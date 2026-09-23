import { join } from "node:path";
import { createServer } from "vite";

import type { BuildCommand } from "../build/BuildCommand.ts";

/**
 * Starts the owned frontend before launching the freshly built desktop shell.
 * A previous server cannot satisfy Tauri's development-server readiness check.
 *
 * @param root Repository root
 * @param config Sidecar configuration for this invocation
 * @param run Caller-owned process runner
 */
export async function runDev(root: string, config: string, run: BuildCommand): Promise<void>
{
    const desktop = join(root, "apps/desktop");
    const server = await createServer({
        configFile: join(desktop, "frontend/vite.config.ts"),
        // Use one address family so localhost cannot resolve to a different server.
        server: { host: "127.0.0.1", strictPort: true }
    });
    try
    {
        await server.listen();
        const devUrl = server.resolvedUrls?.local[0];
        if (!devUrl)
        {
            throw new Error("The owned frontend server did not report its listening URL.");
        }
        server.printUrls();
        const override = JSON.parse(config) as { bundle: { externalBin: string[]; }; };
        await run(
            [
                process.execPath,
                "run",
                "tauri",
                "dev",
                "--config",
                JSON.stringify({ ...override, build: { beforeDevCommand: "", devUrl } })
            ],
            desktop,
            { NANIKA_DEV_REQUIRE_PRIMARY: "1" }
        );
    }
    finally
    {
        await server.close();
    }
}
