import "../runtime.ts";

import { rm } from "node:fs/promises";
import { join } from "node:path";
import { fileURLToPath } from "node:url";

import { buildExtensions } from "./build-extensions.ts";
import { publishBundle } from "./publish-bundle.ts";
import { withBuildTarget } from "./with-build-target.ts";
import { nanikaProcesses } from "../development/processes.ts";

const mode = process.argv[2];
if (mode !== "dev" && mode !== "build")
{
    throw new Error("usage: run.ts <dev|build>");
}
const root = fileURLToPath(new URL("../..", import.meta.url));
const desktop = join(root, "apps/desktop");
const profile = mode === "build" ? "release" : "debug";

await withBuildTarget(root, mode, async (target, run) =>
{
    if (mode === "dev" && nanikaProcesses().length !== 0)
    {
        throw new Error("Stop the running Nanika instance before starting another development mode.");
    }
    const cargoTarget = join(root, "target/cargo");
    const config = await buildExtensions(target, profile, run, cargoTarget);
    if (mode === "dev")
    {
        const { runDev } = await import("../development/run-dev.ts");
        await runDev(root, config, run);
        return;
    }
    const command = ["bun", join(root, "node_modules/@tauri-apps/cli/tauri.js"), "build"];
    // Clear only staging bundles to prevent publishing stale output; retain the cache and published bundle.
    await rm(join(cargoTarget, profile, "bundle"), { recursive: true, force: true });
    command.push("--config", config);
    await run(command, desktop, { NANIKA_DEV_REQUIRE_PRIMARY: "1" });
    await publishBundle(join(cargoTarget, profile, "bundle"), join(root, "target", profile));
});
