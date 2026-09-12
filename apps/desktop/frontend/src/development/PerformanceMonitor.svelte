<script lang="ts">
import { onMount } from "svelte";

import { summarizeFrames } from "./index";
import type { FrameStatistics, SearchObservation } from "./index";

let enabled = $state(false);
let visible = $state(false);
let frames = $state.raw<FrameStatistics | null>(null);
let searchStatus = $state("No sample");
let searchTime = $state<number | null>(null);
let deliveryTime = $state<number | null>(null);
let commitTime = $state<number | null>(null);
let intervals: number[] = [];
let frameId = 0;
let previousFrame: number | null = null;
let lastSummary = 0;
let requestId: number | null = null;
let inputAt = 0;
let receivedAt = 0;
let committedFrames = 0;
let listening = false;

onMount(() =>
{
    updateVisibility();
    return stopSampling;
});

function stopSampling(): void
{
    cancelAnimationFrame(frameId);
    frameId = 0;
    previousFrame = null;
    requestId = null;
    committedFrames = 0;
    if (listening)
    {
        document.removeEventListener("nanika:search-observation", observeSearch);
        listening = false;
    }
}

function startSampling(): void
{
    stopSampling();
    intervals = [];
    frames = null;
    searchStatus = "No sample";
    searchTime = null;
    deliveryTime = null;
    commitTime = null;
    lastSummary = performance.now();
    document.addEventListener("nanika:search-observation", observeSearch);
    listening = true;
    frameId = requestAnimationFrame(sampleFrame);
}

function updateVisibility(): void
{
    const next = document.visibilityState === "visible" && document.hasFocus();
    if (next === visible)
    {
        return;
    }
    visible = next;
    if (enabled && visible)
    {
        startSampling();
    }
    else
    {
        stopSampling();
    }
}

function toggle(): void
{
    enabled = !enabled;
    if (enabled && visible)
    {
        startSampling();
    }
    else
    {
        stopSampling();
    }
}

function handleKeydown(event: KeyboardEvent): void
{
    if (event.isComposing || event.repeat || !visible)
    {
        return;
    }
    if (event.code === "F5" && !event.shiftKey && !event.metaKey && !event.ctrlKey && !event.altKey)
    {
        event.preventDefault();
        toggle();
    }
}

function observeSearch(event: Event): void
{
    const observation = (event as CustomEvent<SearchObservation>).detail;
    if (observation.stage === "input")
    {
        requestId = observation.requestId;
        inputAt = observation.time;
        receivedAt = 0;
        committedFrames = 0;
        searchStatus = "Pending";
        searchTime = null;
        deliveryTime = null;
        commitTime = null;
        return;
    }
    if (observation.requestId !== requestId)
    {
        return;
    }
    if (observation.stage === "rejected" || observation.stage === "failed")
    {
        searchStatus = observation.stage === "rejected" ? "Rejected" : "Failed";
        requestId = null;
        return;
    }
    if (observation.stage === "received" && receivedAt === 0)
    {
        receivedAt = observation.time;
        deliveryTime = receivedAt - inputAt;
    }
    if (observation.stage === "committed" && receivedAt > 0 && committedFrames === 0)
    {
        commitTime = observation.time - receivedAt;
        committedFrames = 2;
    }
}

function sampleFrame(time: number): void
{
    if (previousFrame !== null && time > previousFrame)
    {
        // This is a rolling diagnostic window, not a retained benchmark trace.
        intervals.push(time - previousFrame);
        if (intervals.length > 120)
        {
            intervals.shift();
        }
    }
    previousFrame = time;
    if (committedFrames > 0 && --committedFrames === 0)
    {
        // Two callbacks bracket a rendering opportunity, not physical presentation.
        searchTime = performance.now() - inputAt;
        searchStatus = "Complete";
        requestId = null;
    }
    // Updating the monitor itself every frame would contaminate the observation.
    if (time - lastSummary >= 250)
    {
        frames = summarizeFrames(intervals);
        lastSummary = time;
    }
    frameId = requestAnimationFrame(sampleFrame);
}

function milliseconds(value: number | null): string
{
    return value === null ? "N/A" : `${value.toFixed(1)} ms`;
}
</script>

<svelte:window onkeydown={handleKeydown} onfocus={updateVisibility} onblur={updateVisibility} />
<svelte:document onvisibilitychange={updateVisibility} />

<aside class="monitor" aria-label="Development performance monitor">
    {#if enabled}
        <section id="performance-details" aria-label="Performance measurements">
            <header>
                <strong>Performance</strong>
                <span>{visible ? "Recording" : "Paused"}</span>
            </header>
            <dl>
                <div>
                    <dt>FPS estimate</dt>
                    <dd>{frames ? frames.fps.toFixed(1) : "N/A"}</dd>
                </div>
                <div>
                    <dt>Frame P95 / max</dt>
                    <dd>{milliseconds(frames?.p95 ?? null)} / {milliseconds(frames?.maximum ?? null)}</dd>
                </div>
                <div>
                    <dt>Input → frame estimate</dt>
                    <dd>{searchStatus === "Complete" ? milliseconds(searchTime) : searchStatus}</dd>
                </div>
                <div>
                    <dt>Search + delivery</dt>
                    <dd>{milliseconds(deliveryTime)}</dd>
                </div>
                <div>
                    <dt>DOM commit</dt>
                    <dd>{milliseconds(commitTime)}</dd>
                </div>
            </dl>
            <p>rAF timing · last {frames?.samples ?? 0}/120 intervals<br />60 Hz: 16.7 ms · 120 Hz: 8.3 ms</p>
            <p>Development only. Press F5 to toggle.<br />Not a measurement of display presentation.</p>
        </section>
    {/if}
</aside>

<style>
.monitor {
  position: fixed;
  left: 0.75rem;
  bottom: 0.75rem;
  z-index: 10;
  display: grid;
  justify-items: start;
  gap: 0.5rem;
  color: var(--text-primary);
  font-size: 0.75rem;
  font-variant-numeric: tabular-nums;
  pointer-events: none;
}

section {
  width: 20rem;
  padding: 1rem;
  border: 1px solid var(--border-window);
  border-radius: var(--radius-row);
  background: var(--surface-raised);
  box-shadow: var(--shadow-window);
}

header,
dl > div {
  display: flex;
  justify-content: space-between;
  gap: 0.75rem;
}

header span,
p {
  color: var(--text-secondary);
}

dl {
  display: grid;
  gap: 0.5rem;
  margin: 1rem 0;
}

dd {
  margin: 0;
  text-align: right;
}

p {
  margin: 0.5rem 0 0;
  line-height: 1.5;
}
</style>
