import { mkdir, mkdtemp, rm } from "node:fs/promises";
import { join, resolve } from "node:path";
import { expect, test } from "vitest";

import { runProcessTree } from "../../quality/process-tree.ts";

// Probe the real console state because spawn options alone do not prove native behavior.
test.runIf(process.platform === "win32")("build commands do not expose a visible console window", async () =>
{
    const probe = `
        $ErrorActionPreference = 'Stop'
        Add-Type -TypeDefinition 'using System; using System.Runtime.InteropServices; public static class ConsoleProbe { [DllImport("kernel32.dll")] public static extern IntPtr GetConsoleWindow(); [DllImport("user32.dll")] public static extern bool IsWindowVisible(IntPtr handle); }'
        if ([ConsoleProbe]::IsWindowVisible([ConsoleProbe]::GetConsoleWindow())) { exit 1 }
    `;
    const child = await runProcessTree(
        ["powershell.exe", "-NoLogo", "-NoProfile", "-NonInteractive", "-Command", probe],
        process.cwd(),
        process.env,
        new AbortController().signal
    );
    expect(child.exitCode).toBe(0);
});

test("build command failures retain their exit code", async () =>
{
    const child = await runProcessTree(
        ["bun", "-e", "process.exit(37)"],
        process.cwd(),
        process.env,
        new AbortController().signal
    );
    expect(child.exitCode).toBe(37);
});

test.runIf(process.platform === "win32")("cancelling a hidden command waits for its descendants to exit", async () =>
{
    const root = resolve(import.meta.dirname, "../../../target/test-work");
    await mkdir(root, { recursive: true });
    const fixture = await mkdtemp(join(root, "process-tree-"));
    const cancellation = new AbortController();
    const command = `
        const child = Bun.spawn(["bun", "-e", "setInterval(() => {}, 1000)"], { windowsHide: true });
        await Bun.write("ready.json", JSON.stringify({ parent: process.pid, child: child.pid }));
        await child.exited;
    `;
    const running = runProcessTree(["bun", "-e", command], fixture, process.env, cancellation.signal);
    try
    {
        const ready = Bun.file(join(fixture, "ready.json"));
        const deadline = performance.now() + 5000;
        while (!await Bun.file(join(fixture, "ready.json")).exists())
        {
            if (performance.now() >= deadline)
            {
                throw new Error("Command tree did not become ready.");
            }
            await Bun.sleep(10);
        }
        const pids = await ready.json() as { child: number; parent: number; };
        cancellation.abort("SIGTERM");
        await running;
        for (const pid of [pids.parent, pids.child])
        {
            expect(() => process.kill(pid, 0)).toThrow();
        }
    }
    finally
    {
        cancellation.abort("SIGTERM");
        await running;
        await rm(fixture, { recursive: true });
    }
}, 15_000);
