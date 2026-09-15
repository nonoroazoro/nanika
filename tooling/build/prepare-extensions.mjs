import { createReadStream } from "node:fs";
import { chmod, cp, mkdir, readdir, rm, stat } from "node:fs/promises";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";

import { extensionNames } from "./extension-binaries.mjs";

const profile = process.argv[2];
if (profile !== "debug" && profile !== "release") {
    throw new Error("usage: prepare-extensions.mjs <debug|release>");
}

const root = resolve(fileURLToPath(new URL("../..", import.meta.url)));
const cargoTargetDirectory = process.env.CARGO_TARGET_DIR
    ? resolve(process.cwd(), process.env.CARGO_TARGET_DIR)
    : resolve(root, "target");
const profileDirectory = resolve(cargoTargetDirectory, profile);
const preparedDirectory = resolve(root, "target", "tauri-binaries");
const targetTriple = process.env.TARGET ?? hostTriple();
const executableSuffix = process.platform === "win32" ? ".exe" : "";
const preparedNames = new Set(
    extensionNames.map(name => `${name}-${targetTriple}${executableSuffix}`)
);

await mkdir(preparedDirectory, { recursive: true });
for (const entry of await readdir(preparedDirectory, { withFileTypes: true })) {
    if (!preparedNames.has(entry.name)) {
        await rm(resolve(preparedDirectory, entry.name), { recursive: true, force: true });
    }
}
for (const name of extensionNames) {
    const source = resolve(profileDirectory, `${name}${executableSuffix}`);
    const destination = resolve(preparedDirectory, `${name}-${targetTriple}${executableSuffix}`);
    if (!(await filesMatch(source, destination))) {
        await cp(source, destination);
        if (process.platform === "darwin") {
            const sourceStat = await stat(source);
            await chmod(destination, sourceStat.mode);
        }
    }
}

async function filesMatch(left, right) {
    const [leftStat, rightStat] = await Promise.all([
        stat(left),
        stat(right).catch(error => error.code === "ENOENT" ? null : Promise.reject(error))
    ]);
    if (rightStat === null) {
        return false;
    }
    const modesMatch = process.platform !== "darwin"
        || (leftStat.mode & 0o777) === (rightStat.mode & 0o777);
    return leftStat.size === rightStat.size && modesMatch
        && await hashFile(left) === await hashFile(right);
}

async function hashFile(path) {
    const hash = createHash("sha256");
    for await (const chunk of createReadStream(path)) {
        hash.update(chunk);
    }
    return hash.digest("hex");
}

function hostTriple() {
    const output = execFileSync("rustc", ["-vV"], { encoding: "utf8" });
    const line = output.split("\n").find(value => value.startsWith("host:"));
    if (!line) {
        throw new Error("could not determine the Rust host target triple");
    }
    return line.slice("host:".length).trim();
}
