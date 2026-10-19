import { expect, test } from "vitest";

import inventory from "../../../apps/extensions/built-in/inventory.json";
import { readBuiltInExtensions } from "../../build/built-in-extensions";

test.each(["aarch64-apple-darwin", "x86_64-apple-darwin", "x86_64-pc-windows-msvc"])(
    "%s resolves the distribution inventory to its declared binaries",
    async target =>
    {
        const entries = await readBuiltInExtensions(target);
        expect(entries).toHaveLength(inventory.length);
        expect(new Set(entries.map(entry => entry.manifest.id)).size).toBe(entries.length);
        for (const entry of entries)
        {
            expect(entry.packageName).toBeTruthy();
            expect(entry.manifest.targets[target]?.entrypoint).toContain(entry.binaryName);
            expect(await Bun.file(`${entry.directory}/Cargo.toml`).exists()).toBe(true);
        }
    }
);

test("unsupported targets fail before staging sidecars", async () =>
{
    await expect(readBuiltInExtensions("unsupported-target")).rejects.toThrow("does not support");
});
