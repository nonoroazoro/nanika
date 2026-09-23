import "../runtime.ts";

import { fileURLToPath } from "node:url";

import { withBuildTarget } from "../build/with-build-target.ts";

const root = fileURLToPath(new URL("../..", import.meta.url));
await withBuildTarget(root, "test", async (_target, run) =>
{
    await run([process.execPath, "run", "vitest", "run", ...process.argv.slice(2)]);
});
