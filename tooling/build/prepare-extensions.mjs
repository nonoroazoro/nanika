import { cp, mkdir, rm } from "node:fs/promises";
import { resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { execFileSync } from "node:child_process";

const profile = process.argv[2];
if (profile !== "debug" && profile !== "release") {
    throw new Error("usage: prepare-extensions.mjs <debug|release>");
}

const root = resolve(fileURLToPath(new URL("../..", import.meta.url)));
const profileDirectory = resolve(root, "target", profile);
const preparedDirectory = resolve(root, "target", "tauri-binaries");
const targetTriple = process.env.TARGET ?? hostTriple();
const executableSuffix = process.platform === "win32" ? ".exe" : "";
const names = [
    "nanika-extension-application",
    "nanika-extension-command",
    "nanika-extension-script",
    "nanika-extension-calculator",
    "nanika-extension-clipboard"
];

await rm(preparedDirectory, { recursive: true, force: true });
await mkdir(preparedDirectory, { recursive: true });
for (const name of names) {
    const source = resolve(profileDirectory, `${name}${executableSuffix}`);
    const destination = resolve(preparedDirectory, `${name}-${targetTriple}${executableSuffix}`);
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
