import "../runtime.ts";

import { rm } from "node:fs/promises";
import { join } from "node:path";
import { fileURLToPath } from "node:url";

import { buildExtensions } from "./build-extensions.ts";
import { publishBundle } from "./publish-bundle.ts";
import { withBuildTarget } from "./with-build-target.ts";
import { nanikaProcesses } from "../development/processes.ts";

const mode = process.argv[2];
if (mode !== "dev" && mode !== "build" && mode !== "computer-use")
{
    throw new Error("usage: run.ts <dev|build|computer-use>");
}
if (mode === "computer-use" && process.platform !== "darwin")
{
    throw new Error("Packaged Computer Use development is supported only on macOS; use just dev on Windows.");
}
const root = fileURLToPath(new URL("../..", import.meta.url));
const desktop = join(root, "apps/desktop");
const profile = mode === "build" ? "release" : "debug";

await withBuildTarget(root, mode, async (target, run) =>
{
    if (mode !== "build" && nanikaProcesses().length !== 0)
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
    // A successful invocation may publish only bundles created by that invocation.
    // Keep the compiler cache and the previously published bundle intact.
    await rm(join(cargoTarget, profile, "bundle"), { recursive: true, force: true });
    if (mode === "computer-use")
    {
        command.push("--debug", "--bundles", "app");
    }
    command.push("--config", config);
    await run(command, desktop, { NANIKA_DEV_REQUIRE_PRIMARY: "1" });
    // Publish distributable output separately from the reusable compiler cache.
    await publishBundle(join(cargoTarget, profile, "bundle"), join(root, "target", profile));
    if (mode === "computer-use")
    {
        // Hold the work slot until Launch Services has started the retained bundle.
        await run(["bun", "tooling/development/launch-computer-use-app.ts"]);
    }
});
