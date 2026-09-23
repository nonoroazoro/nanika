import assert from "node:assert/strict";
import { test } from "vitest";

import { orderedViewEvents } from "../../src/bridge/orderedViewEvents.ts";

import type { ViewEvent } from "../../src/types/ViewEvent.ts";
import type { ViewEventRequest } from "../../src/types/ViewEventRequest.ts";

test("selection stays ordered before a captured action and only unsent selections coalesce", async () =>
{
    const sent: ViewEvent[] = [];
    const { promise: first, resolve: finish } = Promise.withResolvers<undefined>();
    const enqueue = orderedViewEvents(async value =>
    {
        assert.ok(value.operation.kind === "event");
        sent.push(value.operation.event);
        if (sent.length === 1)
        {
            await first;
        }
        return { viewRevision: sent.length + 1, navigationRevision: sent.length * 2 };
    });
    const two = enqueue(_request({ kind: "selectionChanged", item_id: "two" }));
    const three = enqueue(_request({ kind: "selectionChanged", item_id: "three" }));
    const four = enqueue(_request({ kind: "selectionChanged", item_id: "four" }));
    const action = enqueue(_request({ kind: "actionInvoked", item_id: "four", action_id: "open" }));
    const five = enqueue(_request({ kind: "selectionChanged", item_id: "five" }));
    assert.equal(await three, null);
    assert.deepEqual(sent, [{ kind: "selectionChanged", item_id: "two" }]);
    finish(undefined);
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
    const sent: ViewEventRequest[] = [];
    const enqueue = orderedViewEvents(async value =>
    {
        sent.push(value);
        if (sent.length === 1)
        {
            throw cause;
        }
        return { viewRevision: 3, navigationRevision: 4 };
    });
    const selection = enqueue(_request({ kind: "selectionChanged", item_id: "two" }));
    const action = enqueue(_request({ kind: "actionInvoked", item_id: "two", action_id: "open" }));
    await assert.rejects(selection, error => error === cause);
    assert.deepEqual(await action, { viewRevision: 3, navigationRevision: 4 });
    assert.equal(sent.length, 2);
});

function _request(event: ViewEvent): ViewEventRequest
{
    return { sessionId: 1, routeId: 1, revision: 1, operation: { kind: "event", event } };
}
