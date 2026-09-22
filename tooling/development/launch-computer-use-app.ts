import { spawnSync } from "node:child_process";
import { existsSync, readFileSync } from "node:fs";
import { setTimeout } from "node:timers/promises";
import { fileURLToPath } from "node:url";

if (process.platform !== "darwin")
{
    throw new Error("Packaged Computer Use development is supported only on macOS; use just dev on Windows.");
}

const app = fileURLToPath(new URL("../../target/debug/bundle/macos/Nanika.app", import.meta.url));
if (!existsSync(app))
{
    throw new Error(`The Computer Use development app was not built at ${app}`);
}
const config: unknown = JSON.parse(
    readFileSync(new URL("../../apps/desktop/shell/tauri.conf.json", import.meta.url), "utf8")
);
if (
    typeof config !== "object" || config === null || !("identifier" in config)
    || typeof config.identifier !== "string"
)
{
    throw new Error("The Tauri configuration must contain a string bundle identifier.");
}
const identity = spawnSync("plutil", ["-extract", "CFBundleIdentifier", "raw", `${app}/Contents/Info.plist`], {
    encoding: "utf8"
});
if (identity.error || identity.status !== 0 || identity.stdout.trim() !== config.identifier)
{
    throw new Error(
        `The Computer Use development app does not have the expected bundle identifier: ${config.identifier}`
    );
}

if (_nanikaProcesses().length !== 0)
{
    throw new Error("Stop the running Nanika instance before launching the Computer Use development app.");
}
console.log(`Launching Computer Use development app: ${app}`);
// Launch Services assigns the bundle identity that Computer Use binds to.
// The debug executable itself rejects a second instance of another build.
const launchResult = spawnSync("open", ["-n", "-a", app], { stdio: "inherit" });
if (launchResult.error)
{
    throw launchResult.error;
}
if (launchResult.status !== 0)
{
    throw new Error(`Launch Services rejected the Computer Use development app: ${launchResult.status}`);
}

const executable = `${app}/Contents/MacOS/nanika-desktop`;
// `open` acknowledges a request before Launch Services starts the executable.
// Bound this development-only readiness check so a failed launch reports failure.
const deadline = performance.now() + 10_000;
let pid: string | undefined;
do
{
    pid = _runningBundleExecutable();
    if (pid)
    {
        break;
    }
    await setTimeout(100);
}
while (performance.now() < deadline);
if (!pid)
{
    throw new Error(`Launch Services did not start the new bundle executable: ${executable}`);
}
console.log(`Verified Computer Use development app process ${pid}: ${executable}`);

function _nanikaProcesses(): string[]
{
    const result = spawnSync("lsof", ["-t", "-c", "nanika-desktop"], { encoding: "utf8" });
    if (result.error || (result.status !== 0 && result.status !== 1))
    {
        throw new Error(`Could not check Nanika processes: ${result.error ?? result.stderr}`);
    }
    return result.stdout.trim().split(/\s+/).filter(Boolean);
}

function _runningBundleExecutable(): string | undefined
{
    for (const processId of _nanikaProcesses())
    {
        const files = spawnSync("lsof", ["-Fn", "-a", "-p", processId, "-d", "txt"], { encoding: "utf8" });
        if (files.error || (files.status !== 0 && files.status !== 1))
        {
            throw new Error(`Could not inspect Nanika process ${processId}: ${files.error ?? files.stderr}`);
        }
        if (files.stdout.split(/\r?\n/).includes(`n${executable}`))
        {
            return processId;
        }
    }
    return undefined;
}
