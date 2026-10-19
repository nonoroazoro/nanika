import "../runtime.ts";

import { mkdir } from "node:fs/promises";
import { join, resolve, sep } from "node:path";
import { createInterface } from "node:readline";
import { fileURLToPath } from "node:url";

import { buildVerificationFrontend } from "./frontend.ts";
import { verifyLauncher } from "./surfaces/launcher.ts";
import { verifySettings } from "./surfaces/settings.ts";
import { verifySyntax } from "./surfaces/syntax.ts";
import { withBuildTarget } from "../build/with-build-target.ts";

const surface = process.argv[2] ?? "settings";
if (!["settings", "launcher", "syntax"].includes(surface))
{
    throw new Error("Usage: just verify-ui <settings|launcher|syntax>");
}
const root = fileURLToPath(new URL("../..", import.meta.url));
const prepared = process.argv[3];
if (prepared)
{
    await _verify(surface, resolve(prepared));
}
else
{
    await withBuildTarget(root, "verification", async (target, run) =>
    {
        const dist = join(target, "frontend");
        await buildVerificationFrontend(root, dist);
        await run(["bun", "tooling/verification/ui.ts", surface, dist], root, {}, "cooperative");
    });
}

async function _verify(selected: string, dist: string): Promise<void>
{
    let cancelled = false;
    const interrupt = () =>
    {
        cancelled = true;
    };
    const control = createInterface({ input: process.stdin });
    control.on("line", value =>
    {
        if (value === "shutdown")
        {
            interrupt();
        }
    });
    control.on("close", interrupt);
    process.on("SIGINT", interrupt);
    process.on("SIGTERM", interrupt);
    const session = `nanika-ui-${process.pid}-${Date.now()}`;
    const output = join(root, "target/verification/ui", session);
    await mkdir(output, { recursive: true });
    const config = join(output, "browser.json");
    await Bun.write(config, JSON.stringify({ headed: false, engine: "chrome", hideScrollbars: false }));
    const server = Bun.serve({
        hostname: "127.0.0.1",
        port: 0,
        async fetch(request)
        {
            const pathname = decodeURIComponent(new URL(request.url).pathname);
            const file = resolve(dist, `.${pathname === "/" ? "/index.html" : pathname}`);
            if (!file.startsWith(`${dist}${sep}`))
            {
                return new Response(null, { status: 403 });
            }
            const content = Bun.file(file);
            return await content.exists() ? new Response(content) : new Response(null, { status: 404 });
        }
    });
    const command = ["agent-browser", "--config", config, "--session", session, "--headed", "false"];
    const environment = {
        ...Object.fromEntries(
            Object.keys(process.env).filter(key => key.startsWith("AGENT_BROWSER_")).map(key => [key, undefined])
        ),
        AGENT_BROWSER_HEADED: "false",
        AGENT_BROWSER_AUTO_CONNECT: "false",
        AGENT_BROWSER_CDP: undefined,
        AGENT_BROWSER_PROVIDER: undefined,
        AGENT_BROWSER_DEFAULT_TIMEOUT: "15000"
    };
    // The session owns the daemon across CLI invocations, not each individual command.
    const browser = async (...args: string[]): Promise<void> =>
    {
        if (cancelled)
        {
            throw new Error("UI verification cancelled");
        }
        const child = Bun.spawn([...command, ...args], {
            cwd: root,
            env: { ...process.env, ...environment },
            windowsHide: true,
            stdin: "ignore",
            stdout: "inherit",
            stderr: "inherit"
        });
        if (await child.exited !== 0)
        {
            throw new Error(`agent-browser ${args[0]} failed`);
        }
    };
    const failures: unknown[] = [];
    try
    {
        await browser("open", `${server.url.href}?surface=${selected}&syntax=on`);
        await browser("set", "viewport", ...(selected === "settings" ? ["920", "680"] : ["760", "520"]));
        await browser("eval", "document.fonts.ready.then(() => true)");
        if (selected === "syntax")
        {
            await browser("wait", "--fn", "window.syntaxReady === true");
            await browser("eval", "window.prepareSyntaxInteraction().then(() => true)");
        }
        for (const theme of ["light", "dark"])
        {
            await browser("set", "media", theme);
            await browser("snapshot", "-i");
            await browser("screenshot", join(output, `${selected}-${theme}.png`));
        }
        const verifiers: Record<string, typeof verifySettings> = {
            settings: verifySettings,
            launcher: verifyLauncher,
            syntax: verifySyntax
        };
        const verify = verifiers[selected];
        if (!verify)
        {
            throw new Error(`Unknown verification surface: ${selected}`);
        }
        await verify(browser, output);
        await browser(
            "eval",
            "(() => { if (window.verificationErrors?.length) throw new Error(window.verificationErrors.join('\\n')); return true; })()"
        );
        await browser("errors");
        await browser(
            "eval",
            `({ userAgent: navigator.userAgent, width: innerWidth, height: innerHeight, scale: devicePixelRatio, visibility: document.visibilityState, reducedMotion: matchMedia('(prefers-reduced-motion: reduce)').matches })`
        );
        if (selected !== "syntax")
        {
            for (const state of ["empty", "loading", "failure"])
            {
                await browser("open", `${server.url.href}?surface=${selected}&state=${state}`);
                if (state === "failure")
                {
                    await browser("wait", "[role=alert]");
                }
                else if (state === "loading")
                {
                    await browser("wait", "[role=status]");
                }
                else if (selected === "settings")
                {
                    await browser(
                        "wait",
                        "--fn",
                        "document.querySelector('[aria-label=\"Settings content\"]')?.getAttribute('aria-busy') === 'false'"
                    );
                }
                else
                {
                    await browser("wait", "--text", "No results");
                }
                await browser("screenshot", join(output, `${selected}-${state}.png`));
                await browser(
                    "eval",
                    "(() => { if (window.verificationErrors.length) throw new Error(window.verificationErrors.join('; ')); return true; })()"
                );
            }
        }
        console.log(`UI verification artifacts: ${output}`);
    }
    catch (error)
    {
        failures.push(error);
        try
        {
            await browser("screenshot", join(output, "failure.png"));
            await browser("snapshot", "-i");
            await browser("errors");
        }
        catch (captureError)
        {
            failures.push(captureError);
        }
    }
    finally
    {
        // Cleanup must still run when the shared command runner has been cancelled.
        try
        {
            const close = Bun.spawnSync([...command, "close"], {
                env: { ...process.env, ...environment },
                timeout: 15_000,
                windowsHide: true
            });
            if (!close.success)
            {
                failures.push(
                    new Error(`Could not close owned browser session ${session}: ${close.stderr.toString()}`)
                );
            }
        }
        catch (error)
        {
            failures.push(error);
        }
        finally
        {
            await server.stop(true);
            process.removeListener("SIGINT", interrupt);
            process.removeListener("SIGTERM", interrupt);
            control.removeAllListeners();
            control.close();
        }
    }
    if (failures.length)
    {
        throw new AggregateError(failures, "UI verification failed");
    }
}
