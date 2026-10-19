import { traceWorkers } from "./instrument-workers";
import { syntaxService } from "../../src/syntax/service";

import type { LatencyFixture } from "./LatencyFixture";

let traceRead: ((stage: string) => void) | null = null;

/**
 * Observe fixture-backed source reads without instrumenting production code.
 *
 * @param stage Read boundary name
 */
export function recordLatencyRead(stage: string): void
{
    traceRead?.(stage);
}

/**
 * Measure actual preview scheduling, Worker messages and DOM publication.
 * Source reads are fixture-backed. rAF timestamps are not physical paint timestamps.
 *
 * @param small First grammar workload
 * @param large Multi-chunk workload in the same grammar family
 * @param rapid Unique document identities used for rapid navigation
 * @param reset Select a plain source between visits
 * @param source Current source container
 */
export async function measureLatency(
    small: LatencyFixture,
    large: LatencyFixture,
    rapid: LatencyFixture[],
    reset: () => void,
    source: () => HTMLElement
)
{
    const visits = [];
    for (
        const [name, fixture] of [["cold-worker", small], ["warm-worker", small], ["multi-chunk", large], [
            "ready-cache",
            small
        ]] as const
    )
    {
        reset();
        if (name !== "ready-cache")
        {
            syntaxService.invalidate("benchmark", fixture.id);
        }
        if (name === "cold-worker")
        {
            syntaxService.suspend();
        }
        await _frame();
        const events: Array<{ milliseconds: number; stage: string; }> = [];
        const started = performance.now();
        const record = (stage: string): void =>
        {
            events.push({ stage, milliseconds: performance.now() - started });
        };
        traceWorkers(record);
        traceRead = record;
        let decorated: number | null = null;
        let finish: (() => void) | null = null;
        let deadline: ReturnType<typeof setTimeout> | undefined;
        const ready = new Promise<void>((resolve, reject) =>
        {
            finish = resolve;
            deadline = setTimeout(() =>
            {
                reject(new Error("Timed out waiting for colors."));
            }, 5000);
        });
        const observe = (): void =>
        {
            if (decorated === null && source().querySelector("[class^=syntax-token-]"))
            {
                decorated = performance.now() - started;
                record("decorated");
                finish?.();
            }
        };
        const observer = new MutationObserver(observe);
        observer.observe(source(), { childList: true, subtree: true });
        try
        {
            fixture.show();
            record("source-commit");
            observe();
            const immediateSource = fixture.text.startsWith(source().textContent ?? "");
            await ready;
            if (decorated === null || !immediateSource)
            {
                throw new Error(`Latency fixture did not publish complete colors: ${fixture.id}`);
            }
            await _frame();
            record("next-frame");
            const time = (stage: string): number | null =>
                events.find(event => event.stage === stage)?.milliseconds ?? null;
            const duration = (from: string, to: string): number | null =>
            {
                const start = time(from);
                const end = time(to);
                return start === null || end === null ? null : end - start;
            };
            if (name === "ready-cache" && (time("worker-send") !== null || time("worker-detect") !== null))
            {
                throw new Error("Ready cache unexpectedly requested background work.");
            }
            const detected = time("worker-detected");
            const read = time("source-read-start");
            if (name !== "ready-cache" && (detected === null || (read !== null && read < detected)))
            {
                throw new Error("Source read started before prefix detection completed.");
            }
            visits.push({
                name,
                fixture: fixture.id,
                immediateSource,
                events,
                selectionToDetection: time("worker-detect"),
                detectionRoundTrip: duration("worker-detect", "worker-detected"),
                selectionToTokenization: time("worker-send"),
                workerInitialization: duration("worker-created", "worker-ready"),
                workerRoundTrip: duration("worker-send", "worker-result"),
                resultToDecoration: duration("worker-result", "decorated"),
                decoration: decorated,
                nextFrame: time("next-frame")
            });
        }
        finally
        {
            clearTimeout(deadline);
            observer.disconnect();
            traceWorkers(null);
            traceRead = null;
        }
    }
    const navigation = [];
    // One, two and four observed 60 Hz frames cover fast repeat through slower browsing.
    // The off/on sequences use the same source and actual rAF cadence.
    for (const frameGap of [1, 2, 4])
    {
        for (const highlighted of [false, true])
        {
            reset();
            syntaxService.suspend();
            for (const fixture of rapid)
            {
                syntaxService.invalidate("benchmark", fixture.id);
            }
            const events: string[] = [];
            traceWorkers(stage =>
            {
                events.push(stage);
            });
            const frames: number[] = [];
            const commits: number[] = [];
            let previous = await _frame();
            let active = true;
            const sample = (): void =>
            {
                if (active)
                {
                    const now = performance.now();
                    frames.push(now - previous);
                    previous = now;
                    requestAnimationFrame(sample);
                }
            };
            requestAnimationFrame(sample);
            try
            {
                for (const fixture of rapid)
                {
                    const start = performance.now();
                    fixture.show(highlighted);
                    source().getBoundingClientRect();
                    commits.push(performance.now() - start);
                    for (let index = 0; index < frameGap; index++)
                    {
                        await _frame();
                    }
                }
            }
            finally
            {
                active = false;
                reset();
                traceWorkers(null);
            }
            navigation.push({
                frameGap,
                highlighted,
                switches: rapid.length,
                frames: _stats(frames.slice(2)),
                commits: _stats(commits),
                workers: events.filter(stage => stage === "worker-created").length,
                jobs: events.filter(stage => stage === "worker-send").length,
                cancellations: events.filter(stage => stage === "worker-cancel").length,
                results: events.filter(stage => stage === "worker-result").length
            });
        }
    }
    return { settleMilliseconds: 0, sourceReads: "fixture-backed, not Rust IPC", visits, navigation };
}

function _stats(values: number[])
{
    const sorted = [...values].sort((a, b) => a - b);
    return {
        p50: sorted[Math.floor(sorted.length * 0.5)],
        p95: sorted[Math.floor(sorted.length * 0.95)],
        max: sorted.at(-1)
    };
}

async function _frame(): Promise<number>
{
    return new Promise(resolve =>
    {
        requestAnimationFrame(resolve);
    });
}
