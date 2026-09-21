import assert from "node:assert/strict";
import { test } from "node:test";

import { viewInputScheduler } from "../../src/bridge/viewInputScheduler.ts";

function navigation(revision, text = "", routeId = 1, busy = false)
{
    return {
        revision,
        busy,
        error: null,
        dismissCount: 0,
        current: { routeId, revision, view: { kind: "list", list: { search_text: text } } }
    };
}

for (const first of ["channel", "rpc"])
{
    void test(`latest query and resume progress exactly once when ${first} finishes first`, () =>
    {
        const input = viewInputScheduler();
        input.update(navigation(1));
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
            input.update(navigation(5, "a"));
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
        input.update(navigation(7, "abc"));
        assert.deepEqual(input.takeNext(), { kind: "resumed" });
        assert.equal(input.takeNext(), null);
    });
}

void test("an unrelated Channel update cannot satisfy an operation's completion receipt", () =>
{
    const input = viewInputScheduler();
    input.update(navigation(1));
    const blocking = input.begin({ kind: "filterChanged", filter_id: "type", value: "text" });
    input.query("latest");
    input.complete(blocking, { viewRevision: 3, navigationRevision: 5 });
    input.update(navigation(3));
    assert.equal(input.busy, true);
    assert.equal(input.takeNext(), null);
    input.update(navigation(5));
    assert.equal(input.busy, false);
    assert.deepEqual(input.takeNext(), { kind: "searchChanged", text: "latest" });
});

void test("a failed query is not retried and a newer explicit query remains eligible", () =>
{
    const input = viewInputScheduler();
    input.update(navigation(1));
    input.query("failed");
    const blocking = input.begin(input.takeNext());
    input.complete(blocking, null);
    input.update({ ...navigation(3), error: "extension rejected query" });
    assert.equal(input.busy, false);
    assert.equal(input.takeNext(), null);
    input.query("new intent");
    assert.deepEqual(input.takeNext(), { kind: "searchChanged", text: "new intent" });
});

void test("route changes discard unsubmitted query and resume intent from the old route", () =>
{
    const input = viewInputScheduler();
    input.update(navigation(1));
    const blocking = input.begin(null);
    input.query("old route query");
    input.resume();
    input.update(navigation(3, "parent", 2));
    input.complete(blocking, { viewRevision: 1, navigationRevision: 3 });
    assert.equal(input.takeNext(), null);
    input.resume();
    assert.deepEqual(input.takeNext(), { kind: "resumed" });
});

void test("selection does not disable input, but queued input waits for its authoritative state", () =>
{
    const input = viewInputScheduler();
    input.update(navigation(1));
    const selection = input.begin({ kind: "selectionChanged", item_id: "two" });
    input.query("latest");
    assert.equal(input.busy, false);
    assert.equal(input.takeNext(), null);
    input.complete(selection, { viewRevision: 2, navigationRevision: 3 });
    assert.equal(input.takeNext(), null);
    input.update(navigation(3));
    assert.deepEqual(input.takeNext(), { kind: "searchChanged", text: "latest" });
});

void test("a coalesced selection does not release an accepted pending action", () =>
{
    const input = viewInputScheduler();
    input.update(navigation(1));
    const selection = input.begin({ kind: "selectionChanged", item_id: "two" });
    const action = input.begin({ kind: "actionInvoked", item_id: "two", action_id: "open" });
    input.query("next");
    input.complete(selection, null);
    assert.equal(input.busy, true);
    assert.equal(input.takeNext(), null);
    input.complete(action, { viewRevision: 2, navigationRevision: 5 });
    input.update(navigation(5));
    assert.equal(input.busy, false);
    assert.deepEqual(input.takeNext(), { kind: "searchChanged", text: "next" });
});

void test("oversized input cannot be submitted and editing it resumes normal dispatch", () =>
{
    const input = viewInputScheduler();
    input.update(navigation(1));
    input.query("x".repeat(4097));
    assert.ok(input.inputError);
    assert.equal(input.takeNext(), null);
    input.query("valid");
    assert.equal(input.inputError, null);
    assert.deepEqual(input.takeNext(), { kind: "searchChanged", text: "valid" });
});
