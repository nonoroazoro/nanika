import { mkdir, mkdtemp, readdir, rm } from "node:fs/promises";
import { join, resolve } from "node:path";
import { expect, test } from "vitest";

import { buildSlotLocks } from "../../build/with-build-lock.ts";
import { withBuildTarget } from "../../build/with-build-target.ts";

test("repeated builds reuse compiler output and preserve unrelated artifacts", async () =>
{
    const fixture = await _fixture();
    try
    {
        await mkdir(join(fixture, "target"));
        await Bun.write(join(fixture, "target/keep"), "retained bundle");
        for (let index = 0; index < 3; index++)
        {
            await withBuildTarget(fixture, "dev", async (target, run) =>
            {
                expect(target).toBe(join(fixture, "target/dev-work"));
                expect(await readdir(target)).toEqual([]);
                expect(await Bun.file(join(fixture, "target/cargo/object")).exists()).toBe(index > 0);
                await run([
                    "bun",
                    "-e",
                    `await Bun.write(process.env.CARGO_TARGET_DIR + "/object", new Uint8Array(1024 * 1024));`
                ]);
                expect(await Bun.file(join(fixture, "target/cargo/object")).exists()).toBe(true);
            });
            expect((await readdir(join(fixture, "target"))).sort()).toEqual([
                buildSlotLocks,
                "cargo",
                "dev-work",
                "keep"
            ]);
        }
    }
    finally
    {
        await rm(fixture, { recursive: true });
    }
});

test("Cargo pipelines exclude each other through staging and reuse cache after the owner exits", async () =>
{
    const fixture = await _fixture();
    try
    {
        await withBuildTarget(fixture, "dev", async (_target, run) =>
        {
            await run(["bun", "-e", 'await Bun.write(process.env.CARGO_TARGET_DIR + "/shared", "cache");']);
            for (const slot of ["dev", "check", "build", "computer-use"] as const)
            {
                await expect(withBuildTarget(fixture, slot, async () =>
                {})).rejects.toThrow("Build output is in use");
            }
            await withBuildTarget(fixture, "test", async () =>
            {});
        });
        await withBuildTarget(fixture, "check", async (target, run) =>
        {
            expect(target).toBe(join(fixture, "target/check-work"));
            await run([
                "bun",
                "-e",
                'if (await Bun.file(process.env.CARGO_TARGET_DIR + "/shared").text() !== "cache") process.exit(1);'
            ]);
        });
    }
    finally
    {
        await rm(fixture, { recursive: true });
    }
});

test("failed work remains reusable", async () =>
{
    const fixture = await _fixture();
    try
    {
        await expect(withBuildTarget(fixture, "dev", async target =>
        {
            await Bun.write(join(target, "partial"), "unfinished output");
            throw new Error("packaging failed");
        })).rejects.toThrow("packaging failed");
        expect(await Bun.file(join(fixture, "target/dev-work/partial")).text()).toBe("unfinished output");
        await withBuildTarget(fixture, "dev", async target =>
        {
            expect(await Bun.file(join(target, "partial")).text()).toBe("unfinished output");
        });
    }
    finally
    {
        await rm(fixture, { recursive: true });
    }
});

test("a forcibly killed owner releases its slot and preserves reusable output", async () =>
{
    const fixture = await _fixture();
    let owner: Bun.Subprocess | undefined;
    try
    {
        await Bun.write(
            join(fixture, "owner.ts"),
            `
            import { withBuildTarget } from ${
                JSON.stringify(resolve(import.meta.dirname, "../../build/with-build-target.ts"))
            };
            await withBuildTarget(import.meta.dirname, "dev", async target => {
                await Bun.write(target + "/partial", "interrupted output");
                await Bun.write("ready", "done");
                setInterval(() => {}, 1000);
                await new Promise(() => {});
            });
        `
        );
        owner = Bun.spawn(["bun", "owner.ts"], { cwd: fixture, stdout: "ignore", stderr: "inherit" });
        const deadline = performance.now() + 5000;
        while (!(await Bun.file(join(fixture, "ready")).exists()))
        {
            if (performance.now() >= deadline)
            {
                throw new Error("Build owner did not become ready.");
            }
            await Bun.sleep(10);
        }
        owner.kill("SIGKILL");
        await owner.exited;
        for (let index = 0; index < 3; index++)
        {
            await withBuildTarget(fixture, "dev", async target =>
            {
                expect(await Bun.file(join(target, "partial")).text()).toBe(
                    index === 0 ? "interrupted output" : "one generation"
                );
                await Bun.write(join(target, "partial"), "one generation");
            });
        }
        expect((await readdir(join(fixture, "target"))).sort()).toEqual([
            buildSlotLocks,
            "cargo",
            "dev-work"
        ]);
    }
    finally
    {
        if (owner)
        {
            owner.kill();
            await owner.exited;
        }
        await rm(fixture, { recursive: true });
    }
});

test.runIf(process.platform === "darwin")(
    "a successful CLI exit stops remaining descendants and leaves output for the next invocation",
    async () =>
    {
        const fixture = await _fixture();
        try
        {
            await Bun.write(
                join(fixture, "descendant.ts"),
                `
            process.once("SIGTERM", async () => {
                await Bun.sleep(50);
                await Bun.write(process.env.CARGO_TARGET_DIR + "/last-write", "done");
                await Bun.write("stopped", "done");
                process.exit(0);
            });
            await Bun.write("ready", "done");
            setInterval(() => {}, 1000);
        `
            );
            await Bun.write(
                join(fixture, "leader.ts"),
                `
            Bun.spawn(["bun", "descendant.ts"], { stdout: "inherit", stderr: "inherit" });
            while (!await Bun.file("ready").exists()) await Bun.sleep(10);
            process.exit(0);
        `
            );
            await withBuildTarget(fixture, "dev", async (_target, run) =>
            {
                await run(["bun", "leader.ts"]);
                expect(await Bun.file(join(fixture, "stopped")).text()).toBe("done");
            });
            expect((await readdir(join(fixture, "target"))).sort()).toEqual([
                buildSlotLocks,
                "cargo",
                "dev-work"
            ]);
            expect(await Bun.file(join(fixture, "target/cargo/last-write")).text()).toBe("done");
        }
        finally
        {
            await rm(fixture, { recursive: true });
        }
    }
);

async function _fixture(): Promise<string>
{
    const root = process.env.CARGO_TARGET_DIR ?? resolve(import.meta.dirname, "../../../target/test-work");
    await mkdir(root, { recursive: true });
    return mkdtemp(join(root, "build-test-"));
}
