import { chmod, mkdir, mkdtemp, readdir, rm } from "node:fs/promises";
import { join, resolve } from "node:path";
import { expect, test } from "vitest";

const repository = resolve(import.meta.dirname, "../..");

test.runIf(process.platform === "darwin").each(
    [
        {
            name: "SIGINT waits for descendants before removing build output",
            signal: "SIGINT",
            inspectionFailure: false
        },
        {
            name: "SIGTERM waits for descendants before removing build output",
            signal: "SIGTERM",
            inspectionFailure: false
        },
        { name: "failed process inspection retains build output", signal: "SIGTERM", inspectionFailure: true }
    ] as const
)(
    "$name",
    async ({ signal, inspectionFailure }) =>
    {
        const fixture = await _fixture();
        let check: Bun.Subprocess<"ignore", "pipe", "pipe"> | undefined;
        const descendants: number[] = [];
        try
        {
            await Bun.write(
                join(fixture, "worker.ts"),
                `
                const child = Bun.spawn([process.execPath, "descendant.ts"], { stdout: "inherit", stderr: "inherit" });
                await child.exited;
            `
            );
            await Bun.write(
                join(fixture, "descendant.ts"),
                `
                const stop = async () => {
                    await Bun.sleep(150);
                    await Bun.write(process.env.CARGO_TARGET_DIR + "/last-write", "done");
                    await Bun.write("stopped", "done");
                    process.exit(0);
                };
                process.once("SIGINT", () => void stop());
                process.once("SIGTERM", () => void stop());
                await Bun.write("ready.json", JSON.stringify({ pid: process.pid, parent: process.ppid }));
                await Bun.write("ready", "done");
                setInterval(() => {}, 1000);
            `
            );
            const environment = { ...process.env };
            if (inspectionFailure)
            {
                const bin = join(fixture, "bin");
                await mkdir(bin);
                await Bun.write(join(bin, "lsof"), "#!/bin/sh\nprintf 'inspection denied' >&2\nexit 42\n");
                await chmod(join(bin, "lsof"), 0o755);
                environment.PATH = `${bin}:${process.env.PATH}`;
            }
            check = Bun.spawn([process.execPath, "tooling/quality/check.ts"], {
                cwd: fixture,
                env: environment,
                stdout: "pipe",
                stderr: "pipe"
            });
            await _waitForFile(join(fixture, "ready"));
            const ready = await Bun.file(join(fixture, "ready.json")).json() as {
                parent: number;
                pid: number;
            };
            descendants.push(ready.pid, ready.parent);
            check.kill(signal);
            const [exitCode, stderr] = await Promise.all([check.exited, new Response(check.stderr).text()]);
            expect(exitCode).not.toBe(0);
            await _waitForFile(join(fixture, "stopped"));
            expect(await Bun.file(join(fixture, "stopped")).text()).toBe("done");
            if (inspectionFailure)
            {
                expect(stderr).toContain("inspection denied");
                expect(await readdir(join(fixture, "target"))).toHaveLength(1);
            }
            else
            {
                expect(stderr).toContain(signal);
                expect(await readdir(join(fixture, "target")), stderr).toEqual([]);
                expect(_running(ready.pid)).toBe(false);
            }
            expect(await Bun.file(join(fixture, "later-stage")).exists()).toBe(false);
        }
        finally
        {
            for (const pid of descendants)
            {
                _kill(pid);
            }
            if (check)
            {
                check.kill();
                await check.exited;
            }
            await rm(fixture, { recursive: true, force: true });
        }
    }
);

test("a failed command preserves its exit cause and stops before later stages", async () =>
{
    const fixture = await _fixture();
    try
    {
        await Bun.write(join(fixture, "worker.ts"), "process.exit(23);");
        const check = Bun.spawn([process.execPath, "tooling/quality/check.ts"], {
            cwd: fixture,
            stdout: "pipe",
            stderr: "pipe"
        });
        const [exitCode, stderr] = await Promise.all([check.exited, new Response(check.stderr).text()]);
        expect(exitCode).not.toBe(0);
        expect(stderr).toContain("failed: 23");
        expect(await readdir(join(fixture, "target"))).toEqual([]);
        expect(await Bun.file(join(fixture, "later-stage")).exists()).toBe(false);
    }
    finally
    {
        await rm(fixture, { recursive: true, force: true });
    }
});

async function _fixture(): Promise<string>
{
    await mkdir(join(repository, "target"), { recursive: true });
    const directory = await mkdtemp(join(repository, "target/check-test-"));
    await mkdir(join(directory, "tooling/quality"), { recursive: true });
    await mkdir(join(directory, "apps/desktop/frontend/src"), { recursive: true });
    await mkdir(join(directory, "engine"));
    for (
        const file of [
            "bunfig.toml",
            "tooling/runtime.ts",
            "tooling/quality/check.ts",
            "tooling/quality/process-tree.ts"
        ]
    )
    {
        await Bun.write(join(directory, file), Bun.file(join(repository, file)));
    }
    await Bun.write(
        join(directory, "package.json"),
        JSON.stringify({
            packageManager: `bun@${Bun.version}`,
            scripts: {
                "extensions:build": "bun worker.ts",
                "format:check": "bun later.ts"
            }
        })
    );
    await Bun.write(join(directory, "later.ts"), "await Bun.write('later-stage', 'unexpected');");
    return directory;
}

async function _waitForFile(path: string): Promise<void>
{
    const deadline = performance.now() + 5000;
    while (!(await Bun.file(path).exists()))
    {
        if (performance.now() >= deadline)
        {
            throw new Error(`Fixture did not become ready: ${path}`);
        }
        await Bun.sleep(10);
    }
}

function _running(pid: number): boolean
{
    try
    {
        process.kill(pid, 0);
        return true;
    }
    catch (error)
    {
        if (error instanceof Error && "code" in error && error.code === "ESRCH")
        {
            return false;
        }
        throw error;
    }
}

function _kill(pid: number): void
{
    try
    {
        process.kill(pid, "SIGKILL");
    }
    catch (error)
    {
        if (!(error instanceof Error && "code" in error && error.code === "ESRCH"))
        {
            throw error;
        }
    }
}
