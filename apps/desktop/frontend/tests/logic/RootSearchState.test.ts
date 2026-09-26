import assert from "node:assert/strict";
import { test } from "vitest";

import { RootSearchState } from "../../src/logic/RootSearchState.svelte";

import type { RootSearchSnapshot, SearchResult } from "../../src/types";

function _result(id: string): SearchResult
{
    return {
        extensionId: "test",
        entryId: id,
        actionId: "open",
        allowDefaultExecution: true,
        title: id,
        subtitle: null,
        iconUrl: null,
        kind: "Extension",
        entryType: "action"
    };
}

function _snapshot(ids: string[], update: Partial<RootSearchSnapshot> = {}): RootSearchSnapshot
{
    return {
        navigation: { revision: 0, current: null, busy: false, error: null, dismissCount: 0 },
        sessionId: 1,
        requestId: 1,
        revision: 1,
        query: "",
        phase: "ready",
        resultRevision: 1,
        resultOffset: 0,
        totalResults: ids.length,
        results: ids.map(_result),
        error: null,
        warnings: [],
        ...update
    };
}

test("pending input keeps the displayed page until the new query's first window arrives", () =>
{
    const page = _snapshot(["a", "b", "c"], { resultOffset: 900, totalResults: 50_000 });
    const state = new RootSearchState(page);
    state.select(901);
    state.scrollTop = 900 * 52;
    const selection = state.selectedResult;
    state.accept(_snapshot([], { requestId: 2, resultRevision: 2, phase: "searching", query: "new" }));
    assert.equal(state.scrollTop, 900 * 52, "pending input must not reset the viewport");
    assert.equal(state.snapshot.results, page.results, "retain the exact displayed payloads");
    assert.equal(state.snapshot.resultOffset, 900);
    assert.equal(state.snapshot.totalResults, 50_000);
    assert.equal(state.selectedResult, selection);
    state.accept(_snapshot(["new-a", "new-b"], { requestId: 2, resultRevision: 3, query: "new" }));
    assert.equal(state.scrollTop, 0, "reset scrolling together with the completed first page");
    assert.equal(state.snapshot.resultOffset, 0);
    assert.equal(state.selectedIndex, 0);
    assert.equal(_selectedEntry(state), "new-a");
});

test("ranking replacement preserves a surviving selection and chooses a row when it disappears", () =>
{
    const state = new RootSearchState(_snapshot(["a", "b", "c"]));
    state.select(1);
    state.accept(_snapshot(["b", "a", "c"], { resultRevision: 2 }));
    assert.equal(state.selectedIndex, 0);
    assert.equal(_selectedEntry(state), "b");
    state.accept(_snapshot(["a", "c"], { resultRevision: 3 }));
    assert.equal(state.selectedIndex, 0);
    assert.equal(_selectedEntry(state), "a");
    state.accept(_snapshot([], { resultRevision: 4 }));
    assert.equal(state.selectedIndex, -1);
    assert.deepEqual(state.selectedResult, null);
    state.accept(_snapshot(["c"], { resultRevision: 5 }));
    assert.equal(_selectedEntry(state), "c");
});

test("a page change preserves an offscreen selection instead of treating it as removed", () =>
{
    const first = _snapshot(["a", "b"], { totalResults: 50_000 });
    const state = new RootSearchState(first);
    state.select(1);
    state.accept(_snapshot(["x", "y"], { resultOffset: 100, totalResults: 50_000 }));
    assert.equal(state.selectedIndex, 1);
    assert.deepEqual(state.selectedResult, null, "an offscreen row is not actionable");
    state.accept(first);
    assert.equal(_selectedEntry(state), "b");
    state.select(100);
    assert.deepEqual(state.selectedResult, null);
    state.accept(_snapshot(["x", "y"], { resultOffset: 100, totalResults: 50_000 }));
    assert.equal(state.selectedIndex, 100);
    assert.equal(_selectedEntry(state), "x");
});

test("replacement selection stays inside a shortened delivered window", () =>
{
    const state = new RootSearchState(_snapshot(["a", "b"], { resultOffset: 100, totalResults: 200 }));
    state.select(101);
    state.accept(_snapshot(["remaining"], { resultOffset: 19, totalResults: 20, resultRevision: 2 }));
    assert.equal(state.selectedIndex, 19);
    assert.equal(_selectedEntry(state), "remaining");
});

test("selection identity includes the action, and an empty query result still resets its viewport", () =>
{
    const first = _snapshot(["a", "a"]);
    const inspection = first.results[1];
    assert.ok(inspection);
    inspection.actionId = "inspect";
    const state = new RootSearchState(first);
    state.select(1);
    state.accept({ ...first, results: [...first.results].reverse(), resultRevision: 2 });
    assert.equal(state.selectedIndex, 0);
    assert.equal(state.selectedResult?.actionId, "inspect");
    state.scrollTop = 520;
    state.accept(_snapshot([], { requestId: 2, resultRevision: 3 }));
    assert.equal(state.scrollTop, 0);
    assert.deepEqual(state.selectedResult, null);
});

function _selectedEntry(state: RootSearchState): string | undefined
{
    return state.selectedResult?.entryId;
}
