import { mkdir, mkdtemp, rm, symlink } from "node:fs/promises";
import { join, resolve } from "node:path";
import { expect, test } from "vitest";

import { publishTypes } from "../../types/output.ts";

test("freshness detects changed, missing and obsolete files without rewriting output", async () =>
{
    const root = await _fixture();
    const source = join(root, "fresh");
    const destination = join(root, "generated");
    try
    {
        await Bun.write(join(source, "Action.ts"), "export type Action = { id: string };\n");
        await Bun.write(join(source, "nested/Event.ts"), "export type Event = 'ready';\n");
        await publishTypes(source, destination, true);
        await Bun.write(join(source, "Action.ts"), "export type Action = { id: string; title: string };\n");
        await Bun.write(join(source, "Added.ts"), "export type Added = boolean;\n");
        const previous = await Bun.file(join(destination, "Action.ts")).text();
        await expect(publishTypes(source, destination, false)).rejects.toThrow("Action.ts, Added.ts, nested/Event.ts");
        expect(await Bun.file(join(destination, "Action.ts")).text()).toBe(previous);
        expect(await Bun.file(join(destination, "nested/Event.ts")).exists()).toBe(true);
        expect(await Bun.file(join(destination, "Added.ts")).exists()).toBe(false);
        await publishTypes(source, destination, true);
        expect(await Bun.file(join(destination, "nested/Event.ts")).exists()).toBe(false);
        await Bun.write(join(source, "Action.ts"), Bun.file(join(destination, "Action.ts")));
        await Bun.write(join(source, "Added.ts"), Bun.file(join(destination, "Added.ts")));
        await expect(publishTypes(source, destination, false)).resolves.toBeUndefined();
    }
    finally
    {
        await rm(root, { recursive: true, force: true });
    }
});

test("empty or unexpected output cannot replace existing generated sources", async () =>
{
    const root = await _fixture();
    try
    {
        const source = join(root, "fresh");
        const destination = join(root, "generated");
        await mkdir(source);
        await Bun.write(join(destination, "Keep.ts"), "keep");
        await expect(publishTypes(source, destination, true)).rejects.toThrow("no files");
        await Bun.write(join(source, "unexpected.txt"), "unexpected");
        await expect(publishTypes(source, destination, true)).rejects.toThrow("Unexpected generated entry");
        expect(await Bun.file(join(destination, "Keep.ts")).text()).toBe("keep");
    }
    finally
    {
        await rm(root, { recursive: true, force: true });
    }
});

test.runIf(process.platform === "darwin")("generated directory symlinks cannot overwrite unrelated files", async () =>
{
    const root = await _fixture();
    try
    {
        await Bun.write(join(root, "fresh/Action.ts"), "fresh");
        await Bun.write(join(root, "outside/Action.ts"), "keep");
        await symlink(join(root, "outside"), join(root, "generated"), "dir");
        await expect(publishTypes(join(root, "fresh"), join(root, "generated"), true)).rejects.toThrow(
            "symbolic links"
        );
        expect(await Bun.file(join(root, "outside/Action.ts")).text()).toBe("keep");
    }
    finally
    {
        await rm(root, { recursive: true, force: true });
    }
});

async function _fixture(): Promise<string>
{
    const target = resolve(import.meta.dirname, "../../../target/test-work");
    await mkdir(target, { recursive: true });
    return mkdtemp(join(target, "types-test-"));
}
