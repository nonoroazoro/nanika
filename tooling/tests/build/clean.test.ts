import { existsSync } from "node:fs";
import { mkdir, mkdtemp, rm } from "node:fs/promises";
import { join, resolve } from "node:path";
import { expect, test } from "vitest";

test("manual clean deletes the entire repository target, ignores cwd and tolerates an absent target", async () =>
{
    const repository = resolve(import.meta.dirname, "../../..");
    const root = process.env.CARGO_TARGET_DIR ?? join(repository, "target/test-work");
    await mkdir(root, { recursive: true });
    const fixture = await mkdtemp(join(root, "clean-test-"));
    try
    {
        for (const file of ["tooling/runtime.ts", "tooling/build/clean.ts"])
        {
            await Bun.write(join(fixture, file), Bun.file(join(repository, file)));
        }
        await Bun.write(join(fixture, "package.json"), JSON.stringify({ packageManager: `bun@${Bun.version}` }));
        await Bun.write(join(fixture, "target/cargo/debug/deps/object"), "cached compiler output");
        await Bun.write(join(fixture, "target/release/bundle/app"), "published bundle");
        await Bun.write(join(fixture, "target/.build-locks/dev.sqlite"), "lock");
        await Bun.write(join(fixture, "unrelated/target/keep"), "unrelated output");
        for (let attempt = 0; attempt < 2; attempt++)
        {
            const child = Bun.spawn(["bun", join(fixture, "tooling/build/clean.ts")], {
                cwd: join(fixture, "unrelated"),
                stdout: "pipe",
                stderr: "pipe"
            });
            const [exitCode, stderr] = await Promise.all([child.exited, new Response(child.stderr).text()]);
            expect(exitCode, stderr).toBe(0);
            expect(existsSync(join(fixture, "target"))).toBe(false);
            expect(await Bun.file(join(fixture, "unrelated/target/keep")).text()).toBe("unrelated output");
        }
    }
    finally
    {
        await rm(fixture, { recursive: true });
    }
});
