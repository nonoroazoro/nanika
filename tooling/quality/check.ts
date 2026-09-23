import "../runtime.ts";

import { existsSync } from "node:fs";
import { join } from "node:path";
import { fileURLToPath } from "node:url";

import { buildExtensions } from "../build/build-extensions.ts";
import { withBuildTarget } from "../build/with-build-target.ts";

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
    for (
        const script of [
            "format:check",
            "lint",
            "frontend:check",
            "tooling:check",
            "frontend:build",
            "test"
        ]
    )
    {
        await run([process.execPath, "run", script]);
    }
    await run(["cargo", "fmt", "--all", "--", "--check"]);
    await run(
        ["cargo", "clippy", "--workspace", "--all-targets", "--locked", "--", "-D", "warnings"],
        root,
        environment
    );
    await run(["cargo", "test", "--workspace", "--all-targets", "--locked"], root, environment);
    await run(["cargo", "doc", "--workspace", "--no-deps", "--locked"], root, environment);
});
