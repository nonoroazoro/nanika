import { mkdir, mkdtemp, rm } from "node:fs/promises";
import { join, resolve } from "node:path";
import { expect, test } from "vitest";

import { OwnedProcess } from "../../quality/OwnedProcess.ts";
import { processRunner, runProcessTree } from "../../quality/process-tree.ts";

test("independent bootstrap leaves a running supervisor executable intact", async () =>
{
    const runner = await processRunner();
    const root = resolve(import.meta.dirname, "../../..");
    const suffix = process.platform === "win32" ? ".exe" : "";
    expect(runner).not.toBe(join(root, "target/cargo/debug", `nanika-process${suffix}`));
    const child = await OwnedProcess.spawn(runner, ["bun", "-e", "await Bun.stdin.text()"], root, process.env, true);
    const bootstrap = Bun.spawn([
        "bun",
        "-e",
        `import { processRunner } from ${JSON.stringify(resolve(import.meta.dirname, "../../quality/process-tree.ts"))};
         console.log(await processRunner());`
    ], {
        cwd: root,
        env: { ...process.env, NANIKA_PROCESS_RUNNER: undefined },
        stdin: "ignore",
        stdout: "pipe",
        stderr: "inherit",
        windowsHide: true
    });
    try
    {
        const output = await new Response(bootstrap.stdout).text();
        expect(await bootstrap.exited).toBe(0);
        expect(output.trim()).toBe(runner);
        expect(child.exitCode).toBeNull();
        await child.control("close");
        await child.exited;
        expect(child.exitCode).toBe(0);
    }
    finally
    {
        await child.control("stop");
        await child.exited;
        await bootstrap.exited;
    }
}, 30000);

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

test("cooperative cancellation lets the runner restore native fixtures before exiting", async () =>
{
    const root = resolve(import.meta.dirname, "../../../target/test-work");
    await mkdir(root, { recursive: true });
    const fixture = await mkdtemp(join(root, "process-cooperative-"));
    const cancellation = new AbortController();
    const command = `
        const stop = Promise.withResolvers();
        const control = (await import("node:readline")).createInterface({ input: process.stdin });
        control.on("line", message => { if (message === "shutdown") stop.resolve(); });
        const owner = Bun.spawn(["bun", "-e", '\
            await Bun.stdin.text();\
            await Bun.write("restored", "original state");\
        '], { stdin: "pipe", windowsHide: true });
        await Bun.write("ready", "ready");
        await stop.promise;
        await owner.stdin.end();
        if (await owner.exited !== 0) throw new Error("fixture restoration failed");
        control.close();
    `;
    const running = runProcessTree(["bun", "-e", command], fixture, process.env, cancellation.signal, "cooperative");
    try
    {
        const deadline = performance.now() + 5000;
        while (!await Bun.file(join(fixture, "ready")).exists())
        {
            if (performance.now() >= deadline)
            {
                throw new Error("Cooperative runner did not become ready");
            }
            await Bun.sleep(10);
        }
        cancellation.abort("SIGINT");
        expect((await running).exitCode).toBe(0);
        expect(await Bun.file(join(fixture, "restored")).text()).toBe("original state");
    }
    finally
    {
        cancellation.abort("SIGTERM");
        await running;
        await rm(fixture, { recursive: true });
    }
}, 15000);

test("cancelling a command waits for its descendants to exit", async () =>
{
    const root = resolve(import.meta.dirname, "../../../target/test-work");
    await mkdir(root, { recursive: true });
    const fixture = await mkdtemp(join(root, "process-tree-"));
    const cancellation = new AbortController();
    const command = `
        const child = Bun.spawn(["bun", "-e", "setInterval(() => {}, 1000)"], { windowsHide: true });
        await Bun.write("ready.tmp", JSON.stringify({ parent: process.pid, child: child.pid }));
        // Existence publishes a complete document, never an in-progress write.
        await (await import("node:fs/promises")).rename("ready.tmp", "ready.json");
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

test.runIf(process.platform === "darwin")(
    "cancellation during descendant cleanup preserves the active shutdown",
    async () =>
    {
        const root = resolve(import.meta.dirname, "../../../target/test-work");
        await mkdir(root, { recursive: true });
        const fixture = await mkdtemp(join(root, "process-cleanup-"));
        const cancellation = new AbortController();
        await Bun.write(
            join(fixture, "child.ts"),
            `
        process.once("SIGTERM", async () => {
            await Bun.write("stopping", "ready");
            while (!await Bun.file("release").exists()) await Bun.sleep(10);
            await Bun.write("stopped", "done");
            process.exit(0);
        });
        await Bun.write("ready", "ready");
        setTimeout(() => process.exit(1), 10000);
    `
        );
        const running = runProcessTree(
            [
                "bun",
                "-e",
                `
        Bun.spawn(["bun", "child.ts"], { stdout: "inherit", stderr: "inherit" });
        while (!await Bun.file("ready").exists()) await Bun.sleep(10);
        process.exit(0);
    `
            ],
            fixture,
            process.env,
            cancellation.signal
        );
        try
        {
            const deadline = performance.now() + 5000;
            while (!await Bun.file(join(fixture, "stopping")).exists())
            {
                if (performance.now() >= deadline)
                {
                    throw new Error("Descendant cleanup did not start.");
                }
                await Bun.sleep(10);
            }
            cancellation.abort("SIGINT");
            await Bun.write(join(fixture, "release"), "continue");
            await running;
            expect(await Bun.file(join(fixture, "stopped")).text()).toBe("done");
        }
        finally
        {
            await Bun.write(join(fixture, "release"), "continue");
            cancellation.abort("SIGTERM");
            await running;
            await rm(fixture, { recursive: true });
        }
    },
    15_000
);

test.each([false, true])(
    "termination kills unresponsive descendants (leader exits first: %s)",
    async leaderExits =>
    {
        const root = resolve(import.meta.dirname, "../../../target/test-work");
        await mkdir(root, { recursive: true });
        const fixture = await mkdtemp(join(root, "process-stubborn-"));
        const cancellation = new AbortController();
        await Bun.write(
            join(fixture, "child.ts"),
            `
            process.on("SIGTERM", () => {});
            await Bun.write("child-ready", "ready");
            setInterval(() => {}, 1000);
        `
        );
        const running = runProcessTree(
            [
                "bun",
                "-e",
                `
            process.on("SIGTERM", () => {});
            const child = Bun.spawn(["bun", "child.ts"], { stdout: "inherit", stderr: "inherit" });
            while (!await Bun.file("child-ready").exists()) await Bun.sleep(10);
            await Bun.write("ready.tmp", JSON.stringify({ parent: process.pid, child: child.pid }));
            await (await import("node:fs/promises")).rename("ready.tmp", "ready.json");
            if (${leaderExits}) process.exit(0);
            await child.exited;
        `
            ],
            fixture,
            process.env,
            cancellation.signal
        );
        try
        {
            const deadline = performance.now() + 5000;
            while (!await Bun.file(join(fixture, "ready.json")).exists())
            {
                if (performance.now() >= deadline)
                {
                    throw new Error("Unresponsive process fixture did not become ready");
                }
                await Bun.sleep(10);
            }
            const pids = await Bun.file(join(fixture, "ready.json")).json() as { child: number; parent: number; };
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
    },
    25000
);
