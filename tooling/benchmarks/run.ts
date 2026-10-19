import "../runtime.ts";

import { mkdir } from "node:fs/promises";
import { join } from "node:path";
import { fileURLToPath } from "node:url";

import { withBuildTarget } from "../build/with-build-target.ts";

const family = process.argv[2];
const args = process.argv.slice(3);
const root = fileURLToPath(new URL("../..", import.meta.url));
if (family === "native")
{
    const { verifyNative } = await import("../verification/native.ts");
    await verifyNative(args[0] ?? "syntax:latency", Number(args[1] ?? "3"));
}
else if (family === "rust" || family === "ts")
{
    await withBuildTarget(root, "benchmark", async (_target, run) =>
    {
        const output = join(root, "target/benchmarks/ts");
        if (family === "ts")
        {
            await mkdir(output, { recursive: true });
        }
        await run(
            family === "rust"
                ? ["cargo", "bench", "--workspace", "--locked", ...args]
                : ["bun", "node_modules/vitest/vitest.mjs", "bench", "--run", "--reporter=default", ...args],
            root,
            { NANIKA_BENCHMARK_OUTPUT: output }
        );
    });
}
else
{
    throw new Error("Usage: just bench <rust|ts|native> [runner arguments]");
}
