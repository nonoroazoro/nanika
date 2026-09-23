import { mkdir, mkdtemp, rm } from "node:fs/promises";
import { join, resolve } from "node:path";
import { expect, test } from "vitest";

import { runDev } from "../../development/run-dev.ts";

test("each development launch serves current frontend source and closes its owned server", async () =>
{
    const fixture = await _fixture(0);
    try
    {
        for (const version of ["old", "new"])
        {
            let url = "";
            await Bun.write(join(fixture, "apps/desktop/frontend/index.html"), `<html><body>${version}</body></html>`);
            await runDev(
                fixture,
                JSON.stringify({ bundle: { externalBin: ["current-sidecar"] } }),
                async (command, cwd, environment) =>
                {
                    const config = JSON.parse(command.at(-1) ?? "") as {
                        build: { beforeDevCommand: string; devUrl: string; };
                        bundle: { externalBin: string[]; };
                    };
                    url = config.build.devUrl;
                    expect(config.build.beforeDevCommand).toBe("");
                    expect(config.bundle.externalBin).toEqual(["current-sidecar"]);
                    expect(cwd).toBe(join(fixture, "apps/desktop"));
                    expect(environment?.NANIKA_DEV_REQUIRE_PRIMARY).toBe("1");
                    expect(await (await fetch(url)).text()).toContain(`<body>${version}</body>`);
                }
            );
            await expect(fetch(url)).rejects.toThrow();
        }
        let url = "";
        await expect(runDev(fixture, "{}", async command =>
        {
            url = (JSON.parse(command.at(-1) ?? "") as { build: { devUrl: string; }; }).build.devUrl;
            throw new Error("shell compilation failed");
        })).rejects.toThrow("shell compilation failed");
        await expect(fetch(url)).rejects.toThrow();
    }
    finally
    {
        await rm(fixture, { recursive: true });
    }
});

test("an old frontend occupying the port prevents native launch instead of being reused", async () =>
{
    const oldServer = Bun.serve({ hostname: "127.0.0.1", port: 0, fetch: () => new Response("old frontend") });
    const port = oldServer.port;
    if (port === undefined)
    {
        throw new Error("The fixture server did not report its port.");
    }
    const fixture = await _fixture(port);
    let launched = false;
    try
    {
        await expect(runDev(fixture, "{}", async () =>
        {
            launched = true;
        })).rejects.toThrow("already in use");
        expect(launched).toBe(false);
        expect(await (await fetch(oldServer.url)).text()).toBe("old frontend");
    }
    finally
    {
        await oldServer.stop(true);
        await rm(fixture, { recursive: true });
    }
});

async function _fixture(port: number): Promise<string>
{
    const root = process.env.CARGO_TARGET_DIR ?? resolve(import.meta.dirname, "../../../target/test-work");
    await mkdir(root, { recursive: true });
    const fixture = await mkdtemp(join(root, "dev-server-test-"));
    await Bun.write(
        join(fixture, "apps/desktop/frontend/vite.config.ts"),
        `export default {
        root: import.meta.dirname,
        server: { port: ${port} },
        optimizeDeps: { noDiscovery: true, include: [] },
        logLevel: "silent"
    };`
    );
    return fixture;
}
