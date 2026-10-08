import "../runtime.ts";

import { fileURLToPath } from "node:url";

import { generateTypes } from "./generate.ts";
import { withBuildTarget } from "../build/with-build-target.ts";

const mode = process.argv[2];
if (mode !== "check" && mode !== "update")
{
    throw new Error("Usage: bun tooling/types/run.ts <check|update>");
}
const root = fileURLToPath(new URL("../..", import.meta.url));
await withBuildTarget(root, "check", async (target, run) =>
{
    await generateTypes(root, target, run, mode === "update");
});
