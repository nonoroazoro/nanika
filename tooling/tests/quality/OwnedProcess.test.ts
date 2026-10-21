import { mkdir, mkdtemp, rm } from "node:fs/promises";
import { join, resolve } from "node:path";
import { expect, test } from "vitest";

import { OwnedProcess } from "../../quality/OwnedProcess.ts";
import { processRunner } from "../../quality/process-tree.ts";

test("closing managed input awaits restoration and repeated stop is harmless", async () =>
{
    const directory = await _directory();
    const child = await OwnedProcess.spawn(
        await processRunner(),
        [
            "bun",
            "-e",
            `
        await Bun.stdin.text();
        await Bun.write("restored", "done");
    `
        ],
        directory,
        process.env,
        true
    );
    try
    {
        await child.control("close");
        await child.exited;
        await child.control("stop");
        expect(child.exitCode).toBe(0);
        expect(await Bun.file(join(directory, "restored")).text()).toBe("done");
    }
    finally
    {
        await child.control("stop");
        await child.exited;
        await rm(directory, { recursive: true });
    }
});

test("a crashed Bun owner revokes its command and descendants through pipe EOF", async () =>
{
    const directory = await _directory();
    const runner = await processRunner();
    const module = resolve(import.meta.dirname, "../../quality/OwnedProcess.ts");
    await Bun.write(
        join(directory, "command.ts"),
        `
        const child = Bun.spawn(["bun", "-e", "setInterval(() => {}, 1000)"], { windowsHide: true });
        await Bun.write("pids", JSON.stringify([process.pid, child.pid]));
        await child.exited;
    `
    );
    await Bun.write(
        join(directory, "owner.ts"),
        `
        import { OwnedProcess } from ${JSON.stringify(module)};
        const child = await OwnedProcess.spawn(${
            JSON.stringify(runner)
        }, ["bun", "command.ts"], import.meta.dirname, process.env);
        while (!await Bun.file("pids").exists()) await Bun.sleep(10);
        process.exit(23);
    `
    );
    // The crashed owner is itself an owned workflow. Wait for its supervisor
    // too: dead command PIDs do not mean Windows has released their cwd.
    const owner = await OwnedProcess.spawn(runner, ["bun", "owner.ts"], directory, process.env, true);
    try
    {
        await owner.exited;
        expect(owner.exitCode).toBe(23);
        const pids = await Bun.file(join(directory, "pids")).json() as number[];
        // On macOS the inner supervisor owns a separate process group, so the
        // outer workflow can finish before that supervisor handles pipe EOF.
        const deadline = performance.now() + 5000;
        while (pids.some(_alive))
        {
            if (performance.now() >= deadline)
            {
                throw new Error("Supervisor left descendants after its owner exited");
            }
            await Bun.sleep(10);
        }
    }
    finally
    {
        await owner.control("stop");
        await owner.exited;
        await rm(directory, { recursive: true });
    }
});

test("managed cleanup is bounded and forced termination is reported as failure", async () =>
{
    const directory = await _directory();
    const child = await OwnedProcess.spawn(
        await processRunner(),
        [
            "bun",
            "-e",
            `
            // Leave cleanup to the Rust Job instead of Bun's own Windows child Job.
            const child = Bun.spawn(["bun", "-e", "setInterval(() => {}, 1000)"], {
                windowsHide: true, detached: process.platform === "win32"
            });
            await Bun.write("pid", String(child.pid));
            process.exit(0);
        `
        ],
        directory,
        process.env,
        true
    );
    try
    {
        await expect(child.exited).rejects.toThrow("process cleanup was not verified");
        const pid = Number(await Bun.file(join(directory, "pid")).text());
        expect(_alive(pid)).toBe(false);
    }
    finally
    {
        await child.control("stop");
        await child.exited.catch(() =>
        {});
        await rm(directory, { recursive: true });
    }
}, 20000);

async function _directory(): Promise<string>
{
    const root = resolve(import.meta.dirname, "../../../target/test-work");
    await mkdir(root, { recursive: true });
    return mkdtemp(join(root, "owned-process-"));
}

function _alive(pid: number): boolean
{
    try
    {
        process.kill(pid, 0);
        return true;
    }
    catch
    {
        return false;
    }
}
