import "../runtime.ts";

import { fileURLToPath } from "node:url";

const pid = process.argv[2];
if (!pid || !/^[1-9]\d*$/.test(pid) || process.argv.length !== 3)
{
    throw new Error("Usage: just activate <pid>");
}
const executable = fileURLToPath(
    new URL(
        `../../target/cargo/debug/nanika-desktop${process.platform === "win32" ? ".exe" : ""}`,
        import.meta.url
    )
);
const result = Bun.spawnSync([executable, "--activate", pid], {
    windowsHide: true,
    stdin: "inherit",
    stdout: "inherit",
    stderr: "inherit"
});
if (!result.success)
{
    throw new Error(`Activation failed (${result.signalCode ?? result.exitCode}).`);
}
