import assert from "node:assert/strict";
import { test } from "vitest";

import { viewInputScheduler } from "../../src/bridge/viewInputScheduler.ts";

import type { NavigationSnapshot } from "../../src/types/NavigationSnapshot.ts";

for (const first of ["channel", "rpc"])
{
    test(`latest query and resume progress exactly once when ${first} finishes first`, () =>
    {
        const input = viewInputScheduler();
        input.update(_navigation(1));
        input.query("a", 30);
        const event = input.takeNext();
        assert.deepEqual(event, { kind: "searchChanged", minimum_items: 30, text: "a" });
        const blocking = input.begin(event);
        input.query("ab", 30);
        input.query("abc", 30);
        input.resume();
        input.resume();
        const channel = () =>
        {
            input.update(_navigation(5, "a"));
        };
        const rpc = () =>
        {
            input.complete(blocking, { viewRevision: 2, navigationRevision: 5 });
        };
        (first === "channel" ? channel : rpc)();
        assert.equal(input.busy, true);
        assert.equal(input.takeNext(), null);
        (first === "channel" ? rpc : channel)();
        assert.equal(input.busy, false);
        const next = input.takeNext();
        assert.deepEqual(next, { kind: "searchChanged", minimum_items: 30, text: "abc" });
        const nextBlocking = input.begin(next);
        assert.equal(input.takeNext(), null);
        input.complete(nextBlocking, { viewRevision: 3, navigationRevision: 7 });
        input.update(_navigation(7, "abc"));
        assert.deepEqual(input.takeNext(), { kind: "resumed" });
        assert.equal(input.takeNext(), null);
    });
}

test("an unrelated Channel update cannot satisfy an operation's completion receipt", () =>
{
    const input = viewInputScheduler();
    input.update(_navigation(1));
    const blocking = input.begin({ kind: "filterChanged", minimum_items: 30, filter_id: "type", value: "text" });
    input.query("latest", 30);
    input.complete(blocking, { viewRevision: 3, navigationRevision: 5 });
    input.update(_navigation(3));
    assert.equal(input.busy, true);
    assert.equal(input.takeNext(), null);
    input.update(_navigation(5));
    assert.equal(input.busy, false);
    assert.deepEqual(input.takeNext(), { kind: "searchChanged", minimum_items: 30, text: "latest" });
});

test("a failed query is not retried and a newer explicit query remains eligible", () =>
{
    const input = viewInputScheduler();
    input.update(_navigation(1));
    input.query("failed", 30);
    const blocking = input.begin(input.takeNext());
    input.complete(blocking, null);
    input.update({ ..._navigation(3), error: "extension rejected query" });
    assert.equal(input.busy, false);
    assert.equal(input.takeNext(), null);
    input.query("new intent", 30);
    assert.deepEqual(input.takeNext(), { kind: "searchChanged", minimum_items: 30, text: "new intent" });
});

test("route changes discard unsubmitted query and resume intent from the old route", () =>
{
    const input = viewInputScheduler();
    input.update(_navigation(1));
    const blocking = input.begin(null);
    input.query("old route query", 30);
    input.resume();
    input.update(_navigation(3, "parent", 2));
    input.complete(blocking, { viewRevision: 1, navigationRevision: 3 });
    assert.equal(input.takeNext(), null);
    input.resume();
    assert.deepEqual(input.takeNext(), { kind: "resumed" });
});

test("selection does not disable input, but queued input waits for its authoritative state", () =>
{
    const input = viewInputScheduler();
    input.update(_navigation(1));
    const selection = input.begin({ kind: "selectionChanged", collection_id: "collection", index: 1 });
    input.query("latest", 30);
    assert.equal(input.busy, false);
    assert.equal(input.takeNext(), null);
    input.complete(selection, { viewRevision: 2, navigationRevision: 3 });
    assert.equal(input.takeNext(), null);
    input.update(_navigation(3));
    assert.deepEqual(input.takeNext(), { kind: "searchChanged", minimum_items: 30, text: "latest" });
});

test("a coalesced selection does not release an accepted pending action", () =>
{
    const input = viewInputScheduler();
    input.update(_navigation(1));
    const selection = input.begin({ kind: "selectionChanged", collection_id: "collection", index: 1 });
    const action = input.begin({ kind: "actionInvoked", invocation: "default", item_id: "two", action_id: "open" });
    input.query("next", 30);
    input.complete(selection, null);
    assert.equal(input.busy, true);
    assert.equal(input.takeNext(), null);
    input.complete(action, { viewRevision: 2, navigationRevision: 5 });
    input.update(_navigation(5));
    assert.equal(input.busy, false);
    assert.deepEqual(input.takeNext(), { kind: "searchChanged", minimum_items: 30, text: "next" });
});

test("oversized input cannot be submitted and editing it resumes normal dispatch", () =>
{
    const input = viewInputScheduler();
    input.update(_navigation(1));
    input.query("x".repeat(4097), 30);
    assert.ok(input.inputError);
    assert.equal(input.takeNext(), null);
    input.query("valid", 30);
    assert.equal(input.inputError, null);
    assert.deepEqual(input.takeNext(), { kind: "searchChanged", minimum_items: 30, text: "valid" });
});

test("unsolicited host work blocks input until its completed Channel state", () =>
{
    const input = viewInputScheduler();
    input.update(_navigation(1, "", 1, true));
    assert.equal(input.busy, true);
    input.query("later", 30);
    assert.equal(input.takeNext(), null);
    input.update(_navigation(2));
    assert.equal(input.busy, false);
    assert.deepEqual(input.takeNext(), { kind: "searchChanged", minimum_items: 30, text: "later" });
});

for (const first of ["rpc", "channel"])
{
    test(`selection then menu blocks duplicate activation until both completions (${first} first)`, () =>
    {
        const input = viewInputScheduler();
        input.update(_navigation(1));
        const selection = input.begin({ kind: "selectionChanged", collection_id: "collection", index: 1 });
        input.update(_navigation(2, "", 1, true));
        // A selection's own busy publication must not disable rapid activation.
        assert.equal(input.busy, false);
        const action = input.begin({ kind: "actionInvoked", invocation: "default", item_id: "two", action_id: "copy" });
        assert.equal(input.busy, true);
        input.complete(selection, { viewRevision: 2, navigationRevision: 3 });
        assert.equal(input.busy, true);
        const rpc = () =>
        {
            input.complete(action, { viewRevision: 2, navigationRevision: 5 });
        };
        const channel = () =>
        {
            input.update(_navigation(5));
        };
        (first === "rpc" ? rpc : channel)();
        assert.equal(input.busy, true);
        (first === "rpc" ? channel : rpc)();
        assert.equal(input.busy, false);
    });
}

test("authoritative query changes are rendered on the same route", () =>
{
    const input = viewInputScheduler();
    input.update(_navigation(1, "needle"));
    assert.equal(input.queryText, "needle");
    const filter = input.begin({ kind: "filterChanged", minimum_items: 30, filter_id: "type", value: "all" });
    input.update(_navigation(3, ""));
    input.complete(filter, { viewRevision: 3, navigationRevision: 3 });
    assert.equal(input.queryText, "");
    input.update(_navigation(2, "stale"));
    assert.equal(input.queryText, "");
});

for (const first of ["channel", "rpc"])
{
    test(`query draft yields to authority after both transports settle (${first} first)`, () =>
    {
        const input = viewInputScheduler();
        input.update(_navigation(1));
        input.query(" draft ", 30);
        const operation = input.begin(input.takeNext());
        const channel = () =>
        {
            input.update(_navigation(5, "draft"));
        };
        const rpc = () =>
        {
            input.complete(operation, { viewRevision: 5, navigationRevision: 5 });
        };
        (first === "channel" ? channel : rpc)();
        assert.equal(input.queryText, " draft ");
        (first === "channel" ? rpc : channel)();
        assert.equal(input.queryText, "draft");
        // A subsequent operation must not resurrect the completed draft.
        input.begin({ kind: "resumed" });
        assert.equal(input.queryText, "draft");
    });

    test(`newer draft survives an older query's authoritative completion (${first} first)`, () =>
    {
        const input = viewInputScheduler();
        input.update(_navigation(1));
        input.query("a", 30);
        const operation = input.begin(input.takeNext());
        input.query("ab", 30);
        const channel = () =>
        {
            input.update(_navigation(5, "a"));
        };
        const rpc = () =>
        {
            input.complete(operation, { viewRevision: 5, navigationRevision: 5 });
        };
        (first === "channel" ? channel : rpc)();
        assert.equal(input.queryText, "ab");
        (first === "channel" ? rpc : channel)();
        assert.equal(input.queryText, "ab");
        assert.deepEqual(input.takeNext(), { kind: "searchChanged", minimum_items: 30, text: "ab" });
    });
}

test("invalid drafts survive publications and disappear with their route", () =>
{
    const input = viewInputScheduler();
    input.update(_navigation(1, "initial"));
    const draft = "x".repeat(4097);
    input.query(draft, 30);
    input.update(_navigation(3, "authoritative"));
    assert.equal(input.queryText, draft);
    assert.ok(input.inputError);
    input.update(_navigation(5, "parent", 2));
    assert.equal(input.queryText, "parent");
    assert.equal(input.inputError, null);
});

test("failed input returns to authority without retry or losing newer input", () =>
{
    const input = viewInputScheduler();
    input.update(_navigation(1, "initial"));
    input.query("failed", 30);
    input.complete(input.begin(input.takeNext()), null);
    assert.equal(input.queryText, "initial");
    assert.equal(input.takeNext(), null);
    input.query("failed again", 30);
    const operation = input.begin(input.takeNext());
    input.query("newer", 30);
    input.complete(operation, null);
    assert.equal(input.queryText, "newer");
    assert.deepEqual(input.takeNext(), { kind: "searchChanged", minimum_items: 30, text: "newer" });
});

test("old route completion cannot restore its draft on the new route", () =>
{
    const input = viewInputScheduler();
    input.update(_navigation(1));
    input.query("old", 30);
    const operation = input.begin(input.takeNext());
    input.update(_navigation(5, "new route", 2));
    input.complete(operation, { viewRevision: 3, navigationRevision: 3 });
    assert.equal(input.queryText, "new route");
    assert.equal(input.takeNext(), null);
});

for (const first of ["channel", "rpc"])
{
    test(`prefetch waits for selection acknowledgement when ${first} arrives first`, () =>
    {
        const input = viewInputScheduler();
        input.update(_navigation(1));
        assert.equal(input.canLoadMore, true);
        const blocking = input.begin({ kind: "selectionChanged", collection_id: "collection", index: 0 });
        assert.equal(input.canLoadMore, false);
        const channel = () =>
        {
            input.update(_navigation(3));
        };
        const rpc = () =>
        {
            input.complete(blocking, { viewRevision: 3, navigationRevision: 3 });
        };
        (first === "channel" ? channel : rpc)();
        assert.equal(input.canLoadMore, false);
        (first === "channel" ? rpc : channel)();
        assert.equal(input.canLoadMore, true);
        const loading = input.begin({ kind: "listRangeChanged", collection_id: "items", offset: 10, count: 30 });
        assert.equal(input.busy, false, "prefetch must not disable selection or input");
        assert.equal(input.canLoadMore, false, "only one prefetch may be in flight");
        input.complete(loading, { viewRevision: 4, navigationRevision: 4 });
        assert.equal(input.canLoadMore, false);
        input.update(_navigation(4));
        assert.equal(input.canLoadMore, true);
    });
}

test("queued search and resume take priority over prefetch", () =>
{
    const input = viewInputScheduler();
    input.update(_navigation(1));
    input.query("unloaded needle", 30);
    assert.equal(input.canLoadMore, false);
    input.complete(input.begin(input.takeNext()), { viewRevision: 2, navigationRevision: 2 });
    input.update(_navigation(2, "unloaded needle"));
    assert.equal(input.canLoadMore, true);
    input.resume();
    assert.equal(input.canLoadMore, false);
});

function _navigation(revision: number, text = "", routeId = 1, busy = false): NavigationSnapshot
{
    return {
        revision,
        busy,
        error: null,
        dismissCount: 0,
        current: {
            routeId,
            instanceId: 1,
            extensionId: "nanika.test",
            generation: 1,
            viewId: "results",
            revision,
            view: {
                kind: "list",
                list: {
                    title: "Results",
                    search_placeholder: "Search",
                    empty_title: "No items",
                    empty_description: "Items will appear here when available.",
                    search_text: text,
                    layout: "plain",
                    sections: [],
                    collection_id: "items",
                    selection: null,
                    detail: null,
                    filter: null
                }
            }
        }
    };
}

test("coalesced queries keep the newest viewport demand with their own text", () =>
{
    const input = viewInputScheduler();
    input.update(_navigation(1));
    const pending = input.begin({ kind: "resumed" });
    input.query("im", 27);
    input.query("image", 43);
    input.complete(pending, { viewRevision: 2, navigationRevision: 3 });
    input.update(_navigation(3));
    assert.deepEqual(input.takeNext(), { kind: "searchChanged", text: "image", minimum_items: 43 });
});

for (const query of ["", "image"])
{
    for (const first of ["channel", "rpc"])
    {
        test(`selection remains nonblocking throughout both transports (${JSON.stringify(query)}, ${first} first)`, () =>
        {
            const input = viewInputScheduler();
            input.update(_navigation(1, query));
            const selection = input.begin({ kind: "selectionChanged", collection_id: "collection", index: 1 });
            input.update(_navigation(2, query, 1, true));
            assert.equal(input.busy, false);
            const channel = () =>
            {
                input.update(_navigation(3, query));
            };
            const rpc = () =>
            {
                input.complete(selection, { viewRevision: 2, navigationRevision: 3 });
            };
            (first === "channel" ? channel : rpc)();
            assert.equal(input.busy, false, "selection must not briefly disable controls between receipt and snapshot");
            assert.equal(input.canLoadMore, false);
            (first === "channel" ? rpc : channel)();
            assert.equal(input.busy, false);
            assert.equal(input.canLoadMore, true);
            // A newer host-owned operation still owns admission.
            input.update(_navigation(4, query, 1, true));
            assert.equal(input.busy, true);
        });
    }
}

for (
    const event of [
        { kind: "resumed" } as const,
        { kind: "listRangeChanged", collection_id: "items", offset: 30, count: 30 } as const
    ]
)
{
    test(`${event.kind} receipt does not expose stale host busy state`, () =>
    {
        const input = viewInputScheduler();
        input.update(_navigation(1, "image"));
        const blocking = input.begin(event);
        input.update(_navigation(2, "image", 1, true));
        input.complete(blocking, { viewRevision: 2, navigationRevision: 3 });
        assert.equal(input.busy, false);
        assert.equal(input.canLoadMore, false);
        input.update(_navigation(3, "image"));
        assert.equal(input.busy, false);
        assert.equal(input.canLoadMore, true);
    });
}
