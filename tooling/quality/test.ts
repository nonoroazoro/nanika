import "../runtime.ts";

import { fileURLToPath } from "node:url";

import { withBuildTarget } from "../build/with-build-target.ts";

const root = fileURLToPath(new URL("../..", import.meta.url));
await withBuildTarget(root, "test", async (_target, run) =>
{
    await run([
        "bun",
        fileURLToPath(new URL("../../node_modules/vitest/vitest.mjs", import.meta.url)),
        "run",
        ...process.argv.slice(2)
    ]);
});
