import { cp, mkdir, readdir, rm } from "node:fs/promises";
import { resolve } from "node:path";
import { execFileSync } from "node:child_process";

const profile = process.argv[2];
if (profile !== "debug" && profile !== "release") {
    throw new Error("usage: stage-extensions.mjs <debug|release>");
}

const root = resolve(new URL("../..", import.meta.url).pathname);
const targetDirectory = resolve(root, "target");
const profileDirectory = resolve(targetDirectory, profile);
const stagingDirectory = resolve(targetDirectory, "tauri-binaries");
const targetTriple = process.env.TARGET ?? hostTriple();
const executableSuffix = process.platform === "win32" ? ".exe" : "";
const names = [
    "nanika-extension-application",
    "nanika-extension-command",
    "nanika-extension-script",
    "nanika-extension-calculator",
    "nanika-extension-clipboard"
];

await rm(stagingDirectory, { recursive: true, force: true });
await mkdir(stagingDirectory, { recursive: true });
for (const name of names) {
    const source = resolve(profileDirectory, `${name}${executableSuffix}`);
    const destination = resolve(stagingDirectory, `${name}-${targetTriple}${executableSuffix}`);
    await cp(source, destination);
}

function hostTriple() {
    const output = execFileSync("rustc", ["-vV"], { encoding: "utf8" });
    const line = output.split("\n").find(value => value.startsWith("host:"));
    if (!line) {
        throw new Error("could not determine the Rust host target triple");
    }
    return line.slice("host:".length).trim();
}
