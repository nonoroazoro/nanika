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
        confirmationTitle: null,
        title: id,
        subtitle: null,
        icon: null,
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
        pendingExtensions: [],
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

test("ordinary activation is immediate and dangerous activation requires two deliberate entries", () =>
{
    const ordinary = _result("app");
    const dangerous = _dangerous();
    const state = new RootSearchState(_snapshot([], { results: [ordinary, dangerous], totalResults: 2 }));
    assert.equal(state.activate(ordinary), "default");
    assert.equal(state.activate(dangerous), null);
    assert.equal(state.confirmationTitle, "Confirm Shut Down");
    assert.equal(state.activate(dangerous), "confirmed");
    assert.equal(state.confirmationTitle, null);
    assert.equal(state.activate(dangerous), null, "completion never arms the next invocation");
});

test("confirmation cannot survive selection, explicit cancellation, pending query or a new result revision", () =>
{
    for (
        const invalidate of [
            (state: RootSearchState) =>
            {
                state.select(1);
                state.select(0);
            },
            (state: RootSearchState) =>
            {
                state.cancelConfirmation();
            },
            (state: RootSearchState) =>
            {
                state.accept({ ...state.snapshot, resultRevision: 2 });
            },
            (state: RootSearchState) =>
            {
                state.accept({ ...state.snapshot, phase: "searching", requestId: 2 });
                state.accept({ ...state.snapshot, phase: "ready" });
            }
        ]
    )
    {
        const dangerous = _dangerous();
        const state = new RootSearchState(_snapshot([], { results: [dangerous, _result("other")], totalResults: 2 }));
        assert.equal(state.activate(dangerous), null);
        invalidate(state);
        assert.equal(state.confirmationTitle, null);
        assert.equal(state.activate(dangerous), null);
        assert.equal(state.confirmationTitle, "Confirm Shut Down");
    }
});

test("hidden, unavailable and stale result objects cannot be confirmed", () =>
{
    const dangerous = _dangerous();
    const disabled = { ..._result("disabled"), allowDefaultExecution: false };
    const state = new RootSearchState(_snapshot([], { results: [dangerous, disabled], totalResults: 2 }));
    assert.equal(state.activate(dangerous), null);
    assert.equal(state.activate({ ...dangerous }), null);
    assert.equal(state.confirmationTitle, null);
    assert.equal(state.activate(disabled), null);
    assert.equal(state.confirmationTitle, null);
    state.accept({ ...state.snapshot, phase: "searching" });
    assert.equal(state.activate(dangerous), null);
});

test("returning to a replaced page or reviewed label cannot restore confirmation", () =>
{
    const dangerous = _dangerous();
    const first = _snapshot([], { results: [dangerous], totalResults: 2 });
    for (
        const replacement of [
            { ...first, results: [_result("other")], resultOffset: 1 },
            { ...first, results: [{ ...dangerous, title: "Changed target" }] },
            { ...first, results: [{ ...dangerous, confirmationTitle: "Changed confirmation" }] },
            { ...first, sessionId: 2 }
        ]
    )
    {
        const state = new RootSearchState(first);
        assert.equal(state.activate(dangerous), null);
        state.accept(replacement);
        assert.equal(state.confirmationTitle, null);
        state.accept(first);
        assert.equal(state.confirmationTitle, null);
        assert.equal(state.activate(dangerous), null, "returning requires a new first activation");
        assert.equal(state.activate(dangerous), "confirmed");
    }
});

function _selectedEntry(state: RootSearchState): string | undefined
{
    return state.selectedResult?.entryId;
}

function _dangerous(): SearchResult
{
    return { ..._result("shutdown"), allowDefaultExecution: false, confirmationTitle: "Confirm Shut Down" };
}

test("progress-only updates preserve reviewed confirmation authority", () =>
{
    const result = { ..._result("power"), allowDefaultExecution: false, confirmationTitle: "Shut down?" };
    const state = new RootSearchState(_snapshot(["power"], { results: [result] }));
    assert.equal(state.activate(result), null);
    assert.equal(state.confirmationTitle, "Shut down?");
    state.accept(_snapshot(["power"], { results: [result], revision: 2, pendingExtensions: ["slow"] }));
    assert.equal(state.confirmationTitle, "Shut down?");
    assert.equal(state.activate(result), "confirmed");
});

test("icon delivery replaces presentation without cancelling reviewed confirmation", () =>
{
    const result = _dangerous();
    const state = new RootSearchState(_snapshot([], { results: [result], totalResults: 1 }));
    assert.equal(state.activate(result), null);
    const decorated: SearchResult = { ...result, icon: { kind: "symbol", name: "power" } };
    state.accept({ ...state.snapshot, revision: 2, results: [decorated] });
    assert.equal(state.selectedResult, decorated);
    assert.equal(state.confirmationTitle, "Confirm Shut Down");
    assert.equal(state.activate(decorated), "confirmed");
});
