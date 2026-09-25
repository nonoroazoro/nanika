import assert from "node:assert/strict";
import { test } from "vitest";

import { SettingsApplications } from "../../src/settings/SettingsApplications.ts";

import type { SettingsApplicationUpdate, SettingsSaveResult } from "../../src/types/Settings.ts";

function _update(requestId: number, result: SettingsSaveResult): SettingsApplicationUpdate
{
    return { extensionId: "test.extension", key: "enabled", requestId, result };
}
function _completed(): SettingsSaveResult
{
    return {
        status: "completed",
        revision: 1,
        values: { enabled: true },
        saved: { enabled: true },
        effective: { enabled: true },
        error: null
    };
}

test("completion before invoke acknowledgement still resolves with backend facts", async () =>
{
    const tracker = new SettingsApplications();
    tracker.record(_update(1, _completed()));
    const result = await tracker.completion(_update(1, { status: "running", progress: null }));
    assert.equal(result.saved.enabled, true);
});

test("progress and stale terminal events cannot release a newer operation", async () =>
{
    const tracker = new SettingsApplications();
    const completion = tracker.completion(_update(2, { status: "running", progress: null }));
    let completed = false;
    void completion.then(() =>
    {
        completed = true;
    });
    tracker.record(_update(1, _completed()));
    tracker.record(_update(2, { status: "running", progress: { label: "Scanning", completed: 10, total: 10 } }));
    await Promise.resolve();
    assert.equal(completed, false, "100 percent is not a terminal acknowledgement");
    tracker.record(_update(2, _completed()));
    await completion;
    assert.equal(tracker.record(_update(2, { status: "running", progress: null })), false);
});

test("terminal transport failure rejects the waiter", async () =>
{
    const tracker = new SettingsApplications();
    const completion = tracker.completion(_update(3, { status: "running", progress: null }));
    tracker.record(_update(3, { status: "failed", error: "Disconnected" }));
    await assert.rejects(completion, /Disconnected/);
});

test("late initial acknowledgement cannot erase early progress", () =>
{
    const tracker = new SettingsApplications();
    assert.equal(
        tracker.record(_update(1, { status: "running", progress: { label: "Scanning", completed: 1, total: 3 } })),
        true
    );
    assert.equal(tracker.record(_update(1, { status: "running", progress: null })), false);
});

test("loading reconciles the snapshot and out-of-order Channel events without regressing completion", async () =>
{
    const tracker = new SettingsApplications();
    tracker.record(_update(5, _completed()));
    tracker.record(_update(5, { status: "running", progress: { label: "Scanning", completed: 2, total: 3 } }));
    tracker.record(_update(5, { status: "running", progress: null }));
    assert.equal(tracker.current("test.extension")?.result.status, "completed");
    const result = await tracker.completion(_update(5, { status: "running", progress: null }));
    assert.equal(result.values.enabled, true);
});
