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
        input.query("a");
        const event = input.takeNext();
        assert.deepEqual(event, { kind: "searchChanged", text: "a" });
        const blocking = input.begin(event);
        input.query("ab");
        input.query("abc");
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
        assert.deepEqual(next, { kind: "searchChanged", text: "abc" });
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
    const blocking = input.begin({ kind: "filterChanged", filter_id: "type", value: "text" });
    input.query("latest");
    input.complete(blocking, { viewRevision: 3, navigationRevision: 5 });
    input.update(_navigation(3));
    assert.equal(input.busy, true);
    assert.equal(input.takeNext(), null);
    input.update(_navigation(5));
    assert.equal(input.busy, false);
    assert.deepEqual(input.takeNext(), { kind: "searchChanged", text: "latest" });
});

test("a failed query is not retried and a newer explicit query remains eligible", () =>
{
    const input = viewInputScheduler();
    input.update(_navigation(1));
    input.query("failed");
    const blocking = input.begin(input.takeNext());
    input.complete(blocking, null);
    input.update({ ..._navigation(3), error: "extension rejected query" });
    assert.equal(input.busy, false);
    assert.equal(input.takeNext(), null);
    input.query("new intent");
    assert.deepEqual(input.takeNext(), { kind: "searchChanged", text: "new intent" });
});

test("route changes discard unsubmitted query and resume intent from the old route", () =>
{
    const input = viewInputScheduler();
    input.update(_navigation(1));
    const blocking = input.begin(null);
    input.query("old route query");
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
    const selection = input.begin({ kind: "selectionChanged", item_id: "two" });
    input.query("latest");
    assert.equal(input.busy, false);
    assert.equal(input.takeNext(), null);
    input.complete(selection, { viewRevision: 2, navigationRevision: 3 });
    assert.equal(input.takeNext(), null);
    input.update(_navigation(3));
    assert.deepEqual(input.takeNext(), { kind: "searchChanged", text: "latest" });
});

test("a coalesced selection does not release an accepted pending action", () =>
{
    const input = viewInputScheduler();
    input.update(_navigation(1));
    const selection = input.begin({ kind: "selectionChanged", item_id: "two" });
    const action = input.begin({ kind: "actionInvoked", invocation: "default", item_id: "two", action_id: "open" });
    input.query("next");
    input.complete(selection, null);
    assert.equal(input.busy, true);
    assert.equal(input.takeNext(), null);
    input.complete(action, { viewRevision: 2, navigationRevision: 5 });
    input.update(_navigation(5));
    assert.equal(input.busy, false);
    assert.deepEqual(input.takeNext(), { kind: "searchChanged", text: "next" });
});

test("oversized input cannot be submitted and editing it resumes normal dispatch", () =>
{
    const input = viewInputScheduler();
    input.update(_navigation(1));
    input.query("x".repeat(4097));
    assert.ok(input.inputError);
    assert.equal(input.takeNext(), null);
    input.query("valid");
    assert.equal(input.inputError, null);
    assert.deepEqual(input.takeNext(), { kind: "searchChanged", text: "valid" });
});

test("unsolicited host work blocks input until its completed Channel state", () =>
{
    const input = viewInputScheduler();
    input.update(_navigation(1, "", 1, true));
    assert.equal(input.busy, true);
    input.query("later");
    assert.equal(input.takeNext(), null);
    input.update(_navigation(2));
    assert.equal(input.busy, false);
    assert.deepEqual(input.takeNext(), { kind: "searchChanged", text: "later" });
});

for (const first of ["rpc", "channel"])
{
    test(`selection then menu blocks duplicate activation until both completions (${first} first)`, () =>
    {
        const input = viewInputScheduler();
        input.update(_navigation(1));
        const selection = input.begin({ kind: "selectionChanged", item_id: "two" });
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
    const filter = input.begin({ kind: "filterChanged", filter_id: "type", value: "all" });
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
        input.query(" draft ");
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
        input.query("a");
        const operation = input.begin(input.takeNext());
        input.query("ab");
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
        assert.deepEqual(input.takeNext(), { kind: "searchChanged", text: "ab" });
    });
}

test("invalid drafts survive publications and disappear with their route", () =>
{
    const input = viewInputScheduler();
    input.update(_navigation(1, "initial"));
    const draft = "x".repeat(4097);
    input.query(draft);
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
    input.query("failed");
    input.complete(input.begin(input.takeNext()), null);
    assert.equal(input.queryText, "initial");
    assert.equal(input.takeNext(), null);
    input.query("failed again");
    const operation = input.begin(input.takeNext());
    input.query("newer");
    input.complete(operation, null);
    assert.equal(input.queryText, "newer");
    assert.deepEqual(input.takeNext(), { kind: "searchChanged", text: "newer" });
});

test("old route completion cannot restore its draft on the new route", () =>
{
    const input = viewInputScheduler();
    input.update(_navigation(1));
    input.query("old");
    const operation = input.begin(input.takeNext());
    input.update(_navigation(5, "new route", 2));
    input.complete(operation, { viewRevision: 3, navigationRevision: 3 });
    assert.equal(input.queryText, "new route");
    assert.equal(input.takeNext(), null);
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
                    search_text: text,
                    layout: "plain",
                    sections: [],
                    selected_item_id: null,
                    detail: null,
                    filter: null,
                    pagination: null
                }
            }
        }
    };
}
