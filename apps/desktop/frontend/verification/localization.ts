import { invoke } from "@tauri-apps/api/core";
import { settled } from "svelte";

import { changeLauncherLanguage } from "./fixtures/tauriBridge";

/**
 * Measure product rendering in the native verification shell with the existing fixture bridge.
 * Storage, extension processes and native input are covered by application E2E, not this workload.
 */
export async function verifyLocalization()
{
    const deadline = performance.now() + 10000;
    while (!document.querySelector('.title[title="Empty Trash"]'))
    {
        if (performance.now() > deadline)
        {
            throw new Error("Localized Launcher did not become ready");
        }
        await _frame();
    }
    const input = document.querySelector<HTMLInputElement>(".launcher input[role=combobox]");
    if (!input)
    {
        throw new Error("Launcher input is missing");
    }
    const launcher = input.closest(".launcher");
    if (!launcher)
    {
        throw new Error("Launcher surface is missing");
    }
    input.focus();
    const rows = [...document.querySelectorAll("[role=option]")];
    const selected = document.querySelector("[role=option][aria-selected=true]");
    const before = _activity();
    const rounds = [];
    for (let round = 0; round < 3; round++)
    {
        const updates = [];
        const frames = [];
        for (let index = 0; index < 100; index++)
        {
            await _frame();
            const locale = index % 2 === 0 ? "zh-Hans" : "en";
            const start = performance.now();
            changeLauncherLanguage(locale);
            await settled();
            updates.push(performance.now() - start);
            const title = locale === "en" ? "Empty Trash" : "清倒废纸篓";
            if (document.documentElement.lang !== locale || !document.querySelector(`.title[title="${title}"]`))
            {
                throw new Error("Locale publication did not settle a coherent presentation");
            }
            if (
                document.querySelector(".launcher input[role=combobox]") !== input
                || document.activeElement !== input || input.value !== ""
                || input.selectionStart !== 0 || input.selectionEnd !== 0
                || document.querySelector("[role=option][aria-selected=true]") !== selected
                || document.querySelectorAll("[role=option]").length !== rows.length
                || [...document.querySelectorAll("[role=option]")].some((row, offset) => row !== rows[offset])
            )
            {
                throw new Error("Repeated switching replaced Launcher state");
            }
            await _frame();
            frames.push(performance.now() - start);
        }
        if (_activity() !== before)
        {
            throw new Error("Warm switching performed resource, query or action work");
        }
        rounds.push({
            updates: _percentiles(updates),
            nextAnimationFrame: _percentiles(frames),
            resources: await invoke("verification_resources", { idleMs: 0, hidden: false })
        });
    }
    let hiddenAt = Infinity;
    let hiddenMutations = 0;
    let observedHidden = false;
    const visibility = (): void =>
    {
        if (document.visibilityState === "hidden")
        {
            observedHidden = true;
            hiddenAt = performance.now();
        }
    };
    const observer = new MutationObserver(records =>
    {
        // Exclude the visibility transition; the native sampler also allows it to settle.
        if (document.visibilityState === "hidden" && performance.now() - hiddenAt > 250)
        {
            hiddenMutations += records.length;
        }
    });
    observer.observe(launcher, {
        subtree: true,
        childList: true,
        characterData: true,
        attributes: true
    });
    document.addEventListener("visibilitychange", visibility);
    let hiddenResources: unknown;
    try
    {
        hiddenResources = await invoke("verification_resources", { idleMs: 2000, hidden: true });
    }
    finally
    {
        observer.disconnect();
        document.removeEventListener("visibilitychange", visibility);
    }
    if (!observedHidden || hiddenMutations !== 0 || _activity() !== before)
    {
        throw new Error("Hidden idle did not remain free of rendering and fixture bridge work");
    }
    if (rounds.some(round => round.updates.p95 > 8))
    {
        throw new Error(
            `Warm locale DOM update exceeds 8 ms p95: ${JSON.stringify(rounds.map(round => round.updates))}`
        );
    }
    return {
        scope:
            "Product Launcher with fixture bridge in release native WebView; locale publication to settled DOM, not OS input or physical presentation latency",
        rounds,
        hiddenMutations,
        hiddenResources,
        activityBefore: before,
        activityAfter: _activity()
    };
}

function _activity(): string
{
    const data = document.documentElement.dataset;
    return JSON.stringify({
        reads: data.localizationReads,
        queries: data.queryCount,
        views: data.viewEventCount,
        actions: data.invocationCount
    });
}

async function _frame(): Promise<void>
{
    return new Promise(resolve =>
    {
        requestAnimationFrame(() =>
        {
            resolve();
        });
    });
}

function _percentiles(values: number[])
{
    const sorted = [...values].sort((left, right) => left - right);
    const at = (percentile: number): number =>
    {
        const value = sorted[Math.ceil(sorted.length * percentile) - 1];
        if (value === undefined)
        {
            throw new Error("Localization measurement has no samples");
        }
        return value;
    };
    return { samples: sorted.length, p50: at(0.5), p95: at(0.95), p99: at(0.99), maximum: at(1) };
}
