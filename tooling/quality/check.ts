import "../runtime.ts";

import { existsSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";

import { buildExtensions } from "../build/build-extensions.ts";
import { withBuildTarget } from "../build/with-build-target.ts";
import { generateTypes } from "../types/generate.ts";

const root = fileURLToPath(new URL("../..", import.meta.url));
const desktop = join(root, "apps/desktop");
if (process.platform !== "darwin" && process.platform !== "win32")
{
    throw new Error(`Unsupported development platform: ${process.platform}`);
}

for (const name of ["crates", "extensions", "scripts", "packaging", "src-tauri", "web", "rust", "dist"])
{
    if (existsSync(join(root, name)))
    {
        throw new Error(`Removed top-level path exists: ${name}`);
    }
}
for await (const path of new Bun.Glob("engine/**/Cargo.toml").scan(root))
{
    if (/^\s*tauri(?:\s|=|-)/m.test(await Bun.file(join(root, path)).text()))
    {
        throw new Error(`Engine crates must not depend on Tauri: ${path}`);
    }
}
for await (const path of new Bun.Glob("**/*.{ts,svelte}").scan(join(desktop, "frontend/src")))
{
    if (
        !path.replaceAll("\\", "/").startsWith("bridge/")
        && (await Bun.file(join(desktop, "frontend/src", path)).text()).includes("@tauri-apps/")
    )
    {
        throw new Error(`Frontend source may import Tauri only through the typed bridge: ${path}`);
    }
}

// Cargo serializes compiler writes; each command stages its own sidecars.
await withBuildTarget(root, "check", async (target, run) =>
{
    const environment = {
        TAURI_CONFIG: await buildExtensions(target, "debug", run, join(root, "target/cargo")),
        RUSTDOCFLAGS: `${process.env.RUSTDOCFLAGS ?? ""} -D warnings`.trim()
    };
    await generateTypes(root, target, run, false);
    await run(["bun", "tooling/syntax/resources.ts", "check"]);
    await run(["bunx", "--no-install", "dprint", "check"], root, {
        DPRINT_CACHE_DIR: join(root, "target/dprint-cache")
    });
    await run(["bun", "run", "lint"]);
    for (const config of ["./tsconfig.json", "./verification/tsconfig.json"])
    {
        await run([
            "bun",
            "node_modules/svelte-check/bin/svelte-check",
            "--workspace",
            "apps/desktop/frontend",
            "--tsconfig",
            config,
            "--fail-on-warnings"
        ]);
    }
    for (
        const project of [
            "tsconfig.json",
            "apps/desktop/frontend/tests/tsconfig.json",
            "apps/desktop/frontend/verification/tsconfig.json",
            "apps/desktop/frontend/benchmarks/tsconfig.json"
        ]
    )
    {
        await run(["bun", "node_modules/typescript/bin/tsc", "--project", project]);
    }
    await run(["bun", "run", "build:frontend"]);
    await run(["bun", "run", "test"]);
    await run(["cargo", "fmt", "--all", "--", "--config", "trailing_comma=Never", "--check"]);
    // Language servers load the base config without the packaging override.
    await run(["cargo", "check", "--workspace", "--all-targets", "--locked"], root, { TAURI_CONFIG: "{}" });
    await run(
        ["cargo", "clippy", "--workspace", "--all-targets", "--locked", "--", "-D", "warnings"],
        root,
        environment
    );
    await run(["cargo", "test", "--workspace", "--lib", "--bins", "--tests", "--locked"], root, environment);
    await run(["cargo", "test", "--workspace", "--doc", "--locked"], root, environment);
    await run(["cargo", "doc", "--workspace", "--no-deps", "--locked"], root, environment);
});
