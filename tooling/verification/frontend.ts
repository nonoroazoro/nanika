import { join } from "node:path";
import { svelte } from "@sveltejs/vite-plugin-svelte";
import { build } from "vite";

/**
 * Builds verification assets separately from the product frontend.
 *
 * @param root Repository root
 * @param output Owned build directory
 */
export async function buildVerificationFrontend(root: string, output: string): Promise<void>
{
    await build({
        configFile: false,
        logLevel: "warn",
        root: join(root, "apps/desktop/frontend/verification"),
        base: "./",
        plugins: [svelte()],
        resolve: {
            alias: [
                {
                    find: /^.*\/tauriBridge$/,
                    replacement: join(root, "apps/desktop/frontend/verification/fixtures/tauriBridge.ts")
                },
                {
                    find: /^.*\/settingsBridge$/,
                    replacement: join(root, "apps/desktop/frontend/verification/fixtures/settingsBridge.ts")
                }
            ]
        },
        publicDir: false,
        build: { outDir: output, emptyOutDir: true, target: "es2022", chunkSizeWarningLimit: 1_024 }
    });
}
