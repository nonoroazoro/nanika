import { mkdir, mkdtemp, rm } from "node:fs/promises";
import { join } from "node:path";

import { publishTypes } from "./output.ts";

import type { BuildCommand } from "../build/BuildCommand.ts";

/**
 * Exports Rust boundary types without rewriting sources during validation.
 *
 * @param root Repository root
 * @param target Caller-owned build target
 * @param run Process runner from {@link withBuildTarget}
 * @param update Whether to replace generated sources
 */
export async function generateTypes(root: string, target: string, run: BuildCommand, update: boolean): Promise<void>
{
    await mkdir(target, { recursive: true });
    const work = await mkdtemp(join(target, "types-"));
    const output = join(work, "generated");
    let completed = false;
    try
    {
        await run(
            [
                "cargo",
                "test",
                "-p",
                "nanika-desktop",
                "--lib",
                "--no-default-features",
                "--features",
                "typescript",
                "--locked",
                "bindings::export",
                "--",
                "--ignored",
                "--exact"
            ],
            root,
            {
                NANIKA_TYPES_OUTPUT: output,
                // Exporting DTOs needs neither packaged sidecars nor frontend assets.
                TAURI_CONFIG: JSON.stringify({ bundle: { externalBin: [], resources: {} } })
            }
        );
        const formatting = await Bun.file(join(root, "dprint.json")).json() as Record<string, unknown>;
        await Bun.write(
            join(work, "dprint.json"),
            JSON.stringify({
                ...formatting,
                includes: ["generated/**/*.ts"],
                excludes: []
            })
        );
        await run(["dprint", "fmt", "--config", join(work, "dprint.json")], work, {
            DPRINT_CACHE_DIR: join(root, "target/dprint-cache")
        });
        await run([
            "bun",
            join(root, "node_modules/typescript/bin/tsc"),
            "--ignoreConfig",
            "--noEmit",
            "--strict",
            "--skipLibCheck",
            "--target",
            "ESNext",
            "--module",
            "Preserve",
            "--types",
            "node",
            join(work, "contracts.ts")
        ]);
        await publishTypes(output, join(root, "apps/desktop/frontend/src/generated"), update);
        completed = true;
    }
    finally
    {
        // Keep failed output for diagnosis and never delete a backup if rollback failed.
        if (completed)
        {
            await rm(work, { recursive: true, force: true });
        }
    }
}
