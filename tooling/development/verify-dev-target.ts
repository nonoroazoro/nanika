import { spawnSync } from "node:child_process";
import { existsSync, rmSync } from "node:fs";
import { fileURLToPath } from "node:url";

if (process.platform === "darwin")
{
    const running = spawnSync("lsof", ["-t", "-c", "nanika-desktop"], { encoding: "utf8" });
    if (running.error || running.stderr || (running.status !== 0 && running.status !== 1))
    {
        throw new Error(`Could not check for a running Nanika instance: ${running.error ?? running.stderr}`);
    }
    if (running.status === 0)
    {
        throw new Error("Stop the running Nanika instance before starting another development mode.");
    }
}
else if (process.platform === "win32")
{
    const running = spawnSync("tasklist", ["/FI", "IMAGENAME eq nanika-desktop.exe", "/FO", "CSV", "/NH"], {
        encoding: "utf8"
    });
    if (running.error || running.status !== 0)
    {
        throw new Error(`Could not check for a running Nanika instance: ${running.error ?? running.stderr}`);
    }
    if (running.stdout.split(/\r?\n/).some(line => /^"nanika-desktop\.exe",/i.test(line)))
    {
        throw new Error("Stop the running Nanika instance before starting another development mode.");
    }
}
else
{
    throw new Error(`Unsupported development platform: ${process.platform}`);
}

// Remove old bundles while retaining Cargo's incremental artifacts.
for (const profile of ["debug", "release"])
{
    const bundleDirectory = fileURLToPath(new URL(`../../target/${profile}/bundle`, import.meta.url));
    if (existsSync(bundleDirectory))
    {
        rmSync(bundleDirectory, { recursive: true });
    }
}
