import { copyFile, mkdir, stat } from "node:fs/promises";
import { basename, join } from "node:path";

import tauriConfig from "../../apps/desktop/shell/tauri.conf.json";

import type { BuildCommand } from "./BuildCommand.ts";

/**
 * Builds and stages the configured sidecars inside the caller's reusable target.
 * Returns the Tauri configuration override consumed by the CLI and Cargo build script.
 *
 * @param target Owned Cargo target directory
 * @param profile Cargo build profile
 * @param run Caller-owned process runner
 * @param cargoTarget Shared Cargo artifact directory; defaults to target for isolated tests
 */
export async function buildExtensions(
    target: string,
    profile: "debug" | "release",
    run: BuildCommand,
    cargoTarget = target
): Promise<string>
{
    const staging = join(target, "sidecars");
    const externalBin = tauriConfig.bundle.externalBin.map(path => join(staging, basename(path)));
    const command = ["cargo", "build", "--locked"];
    if (profile === "release")
    {
        command.push("--release");
    }
    for (const path of externalBin)
    {
        command.push("-p", basename(path));
    }
    await run(command);
    await mkdir(staging, { recursive: true });
    const triple = _hostTriple();
    const suffix = process.platform === "win32" ? ".exe" : "";
    for (const path of externalBin)
    {
        const source = join(cargoTarget, profile, `${basename(path)}${suffix}`);
        const destination = `${path}-${triple}${suffix}`;
        // Unchanged sidecars must keep their timestamps or Tauri rebuilds on every invocation.
        if (!(await _filesMatch(source, destination)))
        {
            await copyFile(source, destination);
        }
    }
    return JSON.stringify({ bundle: { externalBin } });
}

function _hostTriple(): string
{
    const result = Bun.spawnSync(["rustc", "-vV"], { stderr: "pipe", windowsHide: true });
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

async function _filesMatch(source: string, destination: string): Promise<boolean>
{
    const sourceStat = await stat(source);
    let destinationStat;
    try
    {
        destinationStat = await stat(destination);
    }
    catch (error)
    {
        if (error instanceof Error && "code" in error && error.code === "ENOENT")
        {
            return false;
        }
        throw error;
    }
    return sourceStat.size === destinationStat.size
        && (sourceStat.mode & 0o777) === (destinationStat.mode & 0o777)
        && await _hashFile(source) === await _hashFile(destination);
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
