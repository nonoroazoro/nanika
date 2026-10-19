import { invoke } from "@tauri-apps/api/core";

import { syntaxScenarios } from "./syntax/scenarios";

/**
 * Runs only inside the dedicated verification binary, after component mounting.
 */
export async function runNative(): Promise<void>
{
    await invoke("verification_progress", { message: "Frontend started" });
    const surface = await syntaxScenarios.promise;
    await invoke("verification_progress", { message: "Syntax surface mounted" });
    const scenario = new URLSearchParams(location.search).get("scenario");
    try
    {
        await document.fonts.ready;
        await _visible();
        await invoke("verification_progress", { message: `Starting ${scenario}` });
        const before = {
            userAgent: navigator.userAgent,
            viewport: { width: innerWidth, height: innerHeight, scale: devicePixelRatio },
            visibility: document.visibilityState,
            theme: matchMedia("(prefers-color-scheme: dark)").matches ? "dark" : "light",
            reducedMotion: matchMedia("(prefers-reduced-motion: reduce)").matches,
            frameCadence: await _frameCadence()
        };
        const resourcesBefore = await invoke("verification_resources", { idleMs: 0, hidden: false });
        const visibilityChanges: string[] = [];
        const changed = (): void =>
        {
            visibilityChanges.push(document.visibilityState);
        };
        document.addEventListener("visibilitychange", changed);
        let result: unknown;
        try
        {
            if (scenario === "syntax:rendering")
            {
                result = await surface.benchmark();
            }
            else if (scenario === "syntax:latency")
            {
                result = await surface.latency();
            }
            else if (scenario === "syntax:lifecycle")
            {
                const rounds = [];
                for (let round = 0; round < 3; round++)
                {
                    const lifecycle = await surface.lifecycle();
                    const resources = await invoke("verification_resources", { idleMs: 0, hidden: false });
                    rounds.push({ lifecycle, resources });
                }
                const beforeHide = await surface.prepareVisibility();
                const hiddenResources = await invoke("verification_resources", { idleMs: 2000, hidden: true });
                await _visible();
                await new Promise<void>(resolve =>
                {
                    requestAnimationFrame(() =>
                    {
                        resolve();
                    });
                });
                if (!visibilityChanges.includes("hidden"))
                {
                    throw new Error("Native hide did not deliver a visibility change");
                }
                result = { rounds, beforeHide, afterRestore: surface.verifyVisibility(), hiddenResources };
            }
            else
            {
                throw new Error(`Unknown native scenario: ${scenario}`);
            }
        }
        finally
        {
            document.removeEventListener("visibilitychange", changed);
        }
        if (scenario !== "syntax:lifecycle" && visibilityChanges.includes("hidden"))
        {
            throw new Error("Window became hidden during the rendering sample");
        }
        const resourcesAfter = await invoke("verification_resources", { idleMs: 0, hidden: false });
        await invoke("verification_report", {
            report: { environment: before, scenario, result, resourcesBefore, resourcesAfter, visibilityChanges }
        });
    }
    catch (error)
    {
        await invoke("verification_report", { report: { scenario, error: String(error) } });
    }
}

async function _visible(): Promise<void>
{
    if (document.visibilityState === "visible")
    {
        return;
    }
    await new Promise<void>((resolve, reject) =>
    {
        let timer: ReturnType<typeof setTimeout> | undefined = undefined;
        const changed = (): void =>
        {
            if (document.visibilityState === "visible")
            {
                clearTimeout(timer);
                document.removeEventListener("visibilitychange", changed);
                resolve();
            }
        };
        timer = setTimeout(() =>
        {
            document.removeEventListener("visibilitychange", changed);
            reject(new Error("Native rendering requires a visible window"));
        }, 5000);
        document.addEventListener("visibilitychange", changed);
    });
}

/**
 * Observed idle rAF cadence is not a claim about the panel's physical refresh rate.
 */
async function _frameCadence()
{
    const timestamps: number[] = [];
    for (let index = 0; index < 25; index++)
    {
        timestamps.push(
            await new Promise<number>(resolve =>
            {
                requestAnimationFrame(timestamp =>
                {
                    resolve(timestamp);
                });
            })
        );
    }
    const intervals: number[] = [];
    let previous: number | undefined;
    for (const timestamp of timestamps)
    {
        if (previous !== undefined)
        {
            intervals.push(timestamp - previous);
        }
        previous = timestamp;
    }
    intervals.sort((a, b) => a - b);
    return { medianMilliseconds: intervals[Math.floor(intervals.length / 2)], samples: intervals.length };
}
