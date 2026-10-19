import "../runtime.ts";

import { copyFile, mkdir } from "node:fs/promises";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

import { buildExtensions } from "../build/build-extensions.ts";
import { withBuildTarget } from "../build/with-build-target.ts";

const scenario = process.argv[2] ?? "application";
if (!["application", "activation", "ime"].includes(scenario))
{
    throw new Error("Usage: just verify-e2e <application|activation|ime>");
}
const root = fileURLToPath(new URL("../..", import.meta.url));
await withBuildTarget(root, "verification", async (target, run) =>
{
    await run(["bun", "run", "build:frontend"]);
    const config = await buildExtensions(target, "debug", run, join(root, "target/cargo"));
    await run(["cargo", "build", "-p", "nanika-e2e", "--locked"], root, { TAURI_CONFIG: config });
    const resources = (JSON.parse(config) as { bundle: { resources: Record<string, string>; }; }).bundle.resources;
    for (const [source, destination] of Object.entries(resources))
    {
        if (!destination.startsWith("extensions/"))
        {
            continue;
        }
        const path = join(root, "target/cargo/debug", destination);
        await mkdir(dirname(path), { recursive: true });
        await copyFile(source, path);
    }
    await run(["bun", "tooling/verification/e2e/application.ts", scenario], root, {}, "cooperative");
});
