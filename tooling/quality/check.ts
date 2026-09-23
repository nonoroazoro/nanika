import "../runtime.ts";

import { existsSync } from "node:fs";
import { mkdir, mkdtemp, rm } from "node:fs/promises";
import { join } from "node:path";
import { fileURLToPath } from "node:url";

import { runProcessTree } from "./process-tree.ts";

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

// Keep validation builds isolated from a running development application.
await mkdir(join(root, "target"), { recursive: true });
const target = await mkdtemp(join(root, "target/check-"));
const environment = {
    ...process.env,
    CARGO_TARGET_DIR: target,
    CARGO_INCREMENTAL: "0",
    RUSTDOCFLAGS: `${process.env.RUSTDOCFLAGS ?? ""} -D warnings`.trim(),
    ...(process.platform === "darwin" ? { MACOSX_DEPLOYMENT_TARGET: "13.0" } : {})
};
const cancellation = new AbortController();
const interrupt = () =>
{
    cancellation.abort("SIGINT");
};
const terminate = () =>
{
    cancellation.abort("SIGTERM");
};
let targetInUse = false;
process.on("SIGINT", interrupt);
process.on("SIGTERM", terminate);
try
{
    for (
        const script of [
            "extensions:build",
            "format:check",
            "lint",
            "frontend:check",
            "tooling:check",
            "frontend:build",
            "test"
        ]
    )
    {
        await _run([process.execPath, "run", script]);
    }
    await _run(["cargo", "fmt", "--all", "--", "--check"]);
    await _run(["cargo", "clippy", "--workspace", "--all-targets", "--locked", "--", "-D", "warnings"]);
    await _run(["cargo", "test", "--workspace", "--all-targets", "--locked"]);
    await _run(["cargo", "doc", "--workspace", "--no-deps", "--locked"]);
}
finally
{
    // Retain compiler output when process termination could not be verified.
    if (!targetInUse)
    {
        await rm(target, { recursive: true, force: true });
    }
    process.removeListener("SIGINT", interrupt);
    process.removeListener("SIGTERM", terminate);
}

async function _run(command: string[]): Promise<void>
{
    cancellation.signal.throwIfAborted();
    targetInUse = true;
    const child = await runProcessTree(command, root, environment, cancellation.signal);
    targetInUse = false;
    if (cancellation.signal.aborted || child.exitCode !== 0)
    {
        throw new Error(
            `${command.join(" ")} failed: ${cancellation.signal.reason ?? child.signalCode ?? child.exitCode}`
        );
    }
}
