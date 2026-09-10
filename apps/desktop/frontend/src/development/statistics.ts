import type { FrameStatistics } from "./index";

/**
 * Summarize the monitor's explicitly bounded frame interval window.
 *
 * @param intervals Positive intervals between consecutive visible frame callbacks.
 */
export function summarizeFrames(intervals: readonly number[]): FrameStatistics | null
{
    const sorted = [...intervals].sort((left, right) => left - right);
    const p95 = sorted[Math.ceil(sorted.length * 0.95) - 1];
    const maximum = sorted.at(-1);
    if (p95 === undefined || maximum === undefined)
    {
        return null;
    }
    const total = intervals.reduce((sum, interval) => sum + interval, 0);
    return {
        fps: 1000 * intervals.length / total,
        p95,
        maximum,
        samples: intervals.length
    };
}
