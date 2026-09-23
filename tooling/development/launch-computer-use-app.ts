import "../runtime.ts";

import { existsSync } from "node:fs";
import { fileURLToPath } from "node:url";

import { nanikaProcesses } from "./processes.ts";

if (process.platform !== "darwin")
{
    throw new Error("Packaged Computer Use development is supported only on macOS; use just dev on Windows.");
}

const app = fileURLToPath(new URL("../../target/debug/bundle/macos/Nanika.app", import.meta.url));
if (!existsSync(app))
{
    throw new Error(`The Computer Use development app was not built at ${app}`);
}
const config: unknown = await Bun.file(new URL("../../apps/desktop/shell/tauri.conf.json", import.meta.url)).json();
if (
    typeof config !== "object" || config === null || !("identifier" in config)
    || typeof config.identifier !== "string"
)
{
    throw new Error("The Tauri configuration must contain a string bundle identifier.");
}
const identity = Bun.spawnSync(["plutil", "-extract", "CFBundleIdentifier", "raw", `${app}/Contents/Info.plist`], {
    stderr: "pipe"
});
if (!identity.success)
{
    throw new Error(
        `Could not read the bundle identifier (${
            identity.signalCode ?? identity.exitCode
        }): ${identity.stderr.toString()}`
    );
}
if (identity.stdout.toString().trim() !== config.identifier)
{
    throw new Error(
        `The Computer Use development app does not have the expected bundle identifier: ${config.identifier}`
    );
}

if (nanikaProcesses().length !== 0)
{
    throw new Error("Stop the running Nanika instance before launching the Computer Use development app.");
}
console.log(`Launching Computer Use development app: ${app}`);
// Launch Services assigns the bundle identity that Computer Use binds to.
// The debug executable itself rejects a second instance of another build.
const launchResult = Bun.spawnSync(["open", "-n", "-a", app], {
    stdin: "inherit",
    stdout: "inherit",
    stderr: "inherit"
});
if (!launchResult.success)
{
    throw new Error(
        `Launch Services rejected the Computer Use development app: ${launchResult.signalCode ?? launchResult.exitCode}`
    );
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
    await Bun.sleep(100);
}
while (performance.now() < deadline);
if (!pid)
{
    throw new Error(`Launch Services did not start the new bundle executable: ${executable}`);
}
console.log(`Verified Computer Use development app process ${pid}: ${executable}`);

function _runningBundleExecutable(): string | undefined
{
    for (const processId of nanikaProcesses())
    {
        const files = Bun.spawnSync(["lsof", "-Fn", "-a", "-p", processId, "-d", "txt"], { stderr: "pipe" });
        if ((files.exitCode !== 0 && files.exitCode !== 1) || files.stderr.length > 0)
        {
            throw new Error(
                `Could not inspect Nanika process ${processId} (${
                    files.signalCode ?? files.exitCode
                }): ${files.stderr.toString()}`
            );
        }
        if (files.stdout.toString().split(/\r?\n/).includes(`n${executable}`))
        {
            return processId;
        }
    }
    return undefined;
}
