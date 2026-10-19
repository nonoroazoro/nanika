import "../runtime.ts";

import { mkdir } from "node:fs/promises";
import { join } from "node:path";
import { fileURLToPath } from "node:url";

import { buildVerificationFrontend } from "./frontend.ts";
import { withBuildTarget } from "../build/with-build-target.ts";

/**
 * Builds one optimized native harness and runs isolated samples.
 *
 * @param scenario Native workload
 * @param repetitions Fresh-process samples
 */
export async function verifyNative(scenario: string, repetitions = 1): Promise<void>
{
    if (
        !["syntax:rendering", "syntax:latency", "syntax:lifecycle"].includes(scenario)
        || !Number.isInteger(repetitions) || repetitions < 1 || repetitions > 10
    )
    {
        throw new Error("Expected syntax:rendering, syntax:latency or syntax:lifecycle and 1 to 10 repetitions");
    }
    const root = fileURLToPath(new URL("../..", import.meta.url));
    await withBuildTarget(root, "verification", async (target, run) =>
    {
        const dist = join(target, "frontend");
        await buildVerificationFrontend(root, dist);
        const config = JSON.stringify({ build: { frontendDist: dist } });
        await run(
            [
                "cargo",
                "build",
                "-p",
                "nanika-verification",
                "--bin",
                "nanika-verify",
                "--release",
                "--locked"
            ],
            root,
            { TAURI_CONFIG: config }
        );
        const output = join(root, repetitions > 1 ? "target/benchmarks/native" : "target/verification/native");
        await mkdir(output, { recursive: true });
        for (let index = 0; index < repetitions; index++)
        {
            const directory = join(output, `${scenario.replaceAll(":", "-")}-${Date.now()}-${process.pid}-${index}`);
            console.log(`Native ${scenario}: visible Tauri WebView required for rendering; output ${directory}`);
            try
            {
                await run([
                    join(root, `target/cargo/release/nanika-verify${process.platform === "win32" ? ".exe" : ""}`),
                    scenario,
                    directory
                ]);
            }
            catch (error)
            {
                const report = Bun.file(join(directory, "report.json"));
                if (await report.exists())
                {
                    console.log(await report.text());
                }
                throw error;
            }
            const report: unknown = await Bun.file(join(directory, "report.json")).json();
            if (!report || typeof report !== "object" || "error" in report)
            {
                throw new Error(`Native verification failed: ${directory}`);
            }
            console.log(`Native ${scenario} passed: ${join(directory, "report.json")}`);
        }
    });
}

if (import.meta.main)
{
    await verifyNative(process.argv[2] ?? "syntax:rendering");
}
