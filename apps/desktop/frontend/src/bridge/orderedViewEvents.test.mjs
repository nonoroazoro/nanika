import assert from "node:assert/strict";
import { test } from "node:test";

import { orderedViewEvents } from "./orderedViewEvents.ts";

function request(event)
{
    return { sessionId: 1, routeId: 1, revision: 1, operation: { kind: "event", event } };
}

test("selection stays ordered before a captured action and only unsent selections coalesce", async () =>
{
    const sent = [];
    let finish;
    const first = new Promise(resolve =>
    {
        finish = resolve;
    });
    const enqueue = orderedViewEvents(async value =>
    {
        sent.push(value.operation.event);
        if (sent.length === 1)
        {
            await first;
        }
        return { viewRevision: sent.length + 1, navigationRevision: sent.length * 2 };
    });
    const two = enqueue(request({ kind: "selectionChanged", item_id: "two" }));
    const three = enqueue(request({ kind: "selectionChanged", item_id: "three" }));
    const four = enqueue(request({ kind: "selectionChanged", item_id: "four" }));
    const action = enqueue(request({ kind: "actionInvoked", item_id: "four", action_id: "open" }));
    const five = enqueue(request({ kind: "selectionChanged", item_id: "five" }));
    assert.equal(await three, null);
    assert.deepEqual(sent, [{ kind: "selectionChanged", item_id: "two" }]);
    finish();
    await Promise.all([two, four, action, five]);
    assert.deepEqual(sent, [
        { kind: "selectionChanged", item_id: "two" },
        { kind: "selectionChanged", item_id: "four" },
        { kind: "actionInvoked", item_id: "four", action_id: "open" },
        { kind: "selectionChanged", item_id: "five" }
    ]);
});

test("a failed request retains its cause and does not lose an accepted following action", async () =>
{
    const cause = new Error("extension rejected selection");
    const sent = [];
    const enqueue = orderedViewEvents(async value =>
    {
        sent.push(value);
        if (sent.length === 1)
        {
            throw cause;
        }
        return { viewRevision: 3, navigationRevision: 4 };
    });
    const selection = enqueue(request({ kind: "selectionChanged", item_id: "two" }));
    const action = enqueue(request({ kind: "actionInvoked", item_id: "two", action_id: "open" }));
    await assert.rejects(selection, error => error === cause);
    assert.deepEqual(await action, { viewRevision: 3, navigationRevision: 4 });
    assert.equal(sent.length, 2);
});
