import "../runtime.ts";

import { chmod, mkdir, stat } from "node:fs/promises";
import { basename, dirname, resolve } from "node:path";
import { fileURLToPath } from "node:url";

import tauriConfig from "../../apps/desktop/shell/tauri.conf.json";

const profile = process.argv[2];
if (profile !== "debug" && profile !== "release")
{
    throw new Error("usage: build-extensions.ts <debug|release>");
}
if (process.platform !== "darwin" && process.platform !== "win32")
{
    throw new Error(`Unsupported development platform: ${process.platform}`);
}

const root = fileURLToPath(new URL("../..", import.meta.url));
const cargoTargetDirectory = process.env.CARGO_TARGET_DIR
    ? resolve(process.cwd(), process.env.CARGO_TARGET_DIR)
    : resolve(root, "target");
const profileDirectory = resolve(cargoTargetDirectory, profile);
const targetTriple = process.env.TARGET ?? _hostTriple();
const executableSuffix = process.platform === "win32" ? ".exe" : "";
const binaries = tauriConfig.bundle.externalBin.map(path => ({
    name: basename(path),
    destination: resolve(root, "apps/desktop/shell", `${path}-${targetTriple}${executableSuffix}`)
}));

const cargoArguments = ["build", "--locked"];
if (profile === "release")
{
    cargoArguments.push("--release");
}
for (const { name } of binaries)
{
    cargoArguments.push("-p", name);
}

const environment = {
    ...process.env,
    CARGO_TARGET_DIR: cargoTargetDirectory,
    ...(process.platform === "darwin"
        ? { MACOSX_DEPLOYMENT_TARGET: process.env.MACOSX_DEPLOYMENT_TARGET ?? "13.0" }
        : {})
};

const build = Bun.spawnSync(["cargo", ...cargoArguments], {
    cwd: root,
    env: environment,
    stdin: "inherit",
    stdout: "inherit",
    stderr: "inherit"
});
if (!build.success)
{
    throw new Error(`Extension build failed: ${build.signalCode ?? build.exitCode}`);
}

// externalBin is the packaging allowlist; other cached build outputs are not bundled.
for (const { name, destination } of binaries)
{
    const source = resolve(profileDirectory, `${name}${executableSuffix}`);
    if (!(await _filesMatch(source, destination)))
    {
        await mkdir(dirname(destination), { recursive: true });
        await Bun.write(destination, Bun.file(source));
        if (process.platform === "darwin")
        {
            const sourceStat = await stat(source);
            await chmod(destination, sourceStat.mode);
        }
    }
}

async function _filesMatch(left: string, right: string): Promise<boolean>
{
    const [leftStat, rightStat] = await Promise.all([
        stat(left),
        stat(right).catch((error: unknown) =>
        {
            if (error instanceof Error && "code" in error && error.code === "ENOENT")
            {
                return null;
            }
            throw error;
        })
    ]);
    if (rightStat === null)
    {
        return false;
    }
    const modesMatch = process.platform !== "darwin"
        || (leftStat.mode & 0o777) === (rightStat.mode & 0o777);
    return leftStat.size === rightStat.size && modesMatch
        && await _hashFile(left) === await _hashFile(right);
}

async function _hashFile(path: string): Promise<string>
{
    const hash = new Bun.CryptoHasher("sha256");
    for await (const chunk of Bun.file(path).stream())
    {
        hash.update(chunk);
    }
    return hash.digest("hex");
}

function _hostTriple(): string
{
    const result = Bun.spawnSync(["rustc", "-vV"], { stderr: "pipe" });
    if (!result.success)
    {
        throw new Error(
            `Could not inspect the Rust host target (${
                result.signalCode ?? result.exitCode
            }): ${result.stderr.toString()}`
        );
    }
    const line = result.stdout.toString().split("\n").find(value => value.startsWith("host:"));
    if (!line)
    {
        throw new Error("Could not determine the Rust host target triple.");
    }
    return line.slice("host:".length).trim();
}
