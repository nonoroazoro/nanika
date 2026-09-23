import { mkdir, mkdtemp, readdir, rm } from "node:fs/promises";
import { join, resolve } from "node:path";
import { expect, test } from "vitest";

import { publishBundle } from "../../build/publish-bundle.ts";

test("successive bundles replace old versions without retaining backups", async () =>
{
    const fixture = await _fixture();
    try
    {
        for (const version of ["1", "2", "3"])
        {
            await Bun.write(join(fixture, `source/app-${version}`), version);
            await publishBundle(join(fixture, "source"), join(fixture, "release"));
            expect(await readdir(join(fixture, "release"))).toEqual(["bundle"]);
            expect(await readdir(join(fixture, "release/bundle"))).toEqual([`app-${version}`]);
        }
    }
    finally
    {
        await rm(fixture, { recursive: true });
    }
});

test("a failed replacement restores the current bundle", async () =>
{
    const fixture = await _fixture();
    try
    {
        await Bun.write(join(fixture, "release/bundle/app"), "current");
        await expect(publishBundle(join(fixture, "missing"), join(fixture, "release"))).rejects.toThrow();
        expect(await Bun.file(join(fixture, "release/bundle/app")).text()).toBe("current");
        expect(await readdir(join(fixture, "release"))).toEqual(["bundle"]);
    }
    finally
    {
        await rm(fixture, { recursive: true });
    }
});

async function _fixture(): Promise<string>
{
    const root = process.env.CARGO_TARGET_DIR ?? resolve(import.meta.dirname, "../../../target/test-work");
    await mkdir(root, { recursive: true });
    return mkdtemp(join(root, "bundle-test-"));
}
