import { basename, join, resolve } from "node:path";

import inventory from "../../apps/extensions/built-in/inventory.json";

/**
 * Resolve sidecars from the same distribution inventory embedded by desktop and CLI.
 *
 * @param target Rust target triple used to select each manifest entrypoint.
 */
export async function readBuiltInExtensions(target: string)
{
    const extensions = await Promise.all(inventory.map(async name =>
    {
        const directory = resolve(import.meta.dirname, "../../apps/extensions/built-in", name);
        const manifest = Bun.JSONC.parse(await Bun.file(join(directory, "manifest.jsonc")).text()) as {
            icon: string;
            id: string;
            targets: Record<string, { entrypoint: string; }>;
        };
        const cargo = Bun.TOML.parse(await Bun.file(join(directory, "Cargo.toml")).text()) as {
            package: { name: string; };
        };
        const entrypoint = manifest.targets[target]?.entrypoint;
        if (!entrypoint)
        {
            throw new Error(`Built-in extension ${manifest.id} does not support ${target}`);
        }
        return { directory, packageName: cargo.package.name, binaryName: basename(entrypoint, ".exe"), manifest };
    }));
    const binaries = new Set(extensions.map(extension => extension.binaryName));
    const identifiers = new Set(extensions.map(extension => extension.manifest.id));
    if (binaries.size !== extensions.length || identifiers.size !== extensions.length)
    {
        throw new Error("Built-in extensions must have unique IDs and sidecar names.");
    }
    return extensions;
}
