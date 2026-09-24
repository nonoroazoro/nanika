import assert from "node:assert/strict";
import { test } from "vitest";

import { orderedViewEvents } from "../../src/bridge/orderedViewEvents.ts";

import type { ContextMenuRequest } from "../../src/types/ContextMenuRequest.ts";
import type { ViewEvent } from "../../src/types/ViewEvent.ts";
import type { ViewEventReceipt } from "../../src/types/ViewEventReceipt.ts";
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
    }, async () =>
    {
        throw new Error("Unexpected menu invocation");
    }).event;
    const two = enqueue(_request({ kind: "selectionChanged", item_id: "two" }));
    const three = enqueue(_request({ kind: "selectionChanged", item_id: "three" }));
    const four = enqueue(_request({ kind: "selectionChanged", item_id: "four" }));
    const action = enqueue(
        _request({ kind: "actionInvoked", invocation: "default", item_id: "four", action_id: "open" })
    );
    const five = enqueue(_request({ kind: "selectionChanged", item_id: "five" }));
    assert.equal(await three, null);
    assert.deepEqual(sent, [{ kind: "selectionChanged", item_id: "two" }]);
    finish(undefined);
    await Promise.all([two, four, action, five]);
    assert.deepEqual(sent, [
        { kind: "selectionChanged", item_id: "two" },
        { kind: "selectionChanged", item_id: "four" },
        { kind: "actionInvoked", invocation: "default", item_id: "four", action_id: "open" },
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
    }, async () =>
    {
        throw new Error("Unexpected menu invocation");
    }).event;
    const selection = enqueue(_request({ kind: "selectionChanged", item_id: "two" }));
    const action = enqueue(
        _request({ kind: "actionInvoked", invocation: "default", item_id: "two", action_id: "open" })
    );
    await assert.rejects(selection, error => error === cause);
    assert.deepEqual(await action, { viewRevision: 3, navigationRevision: 4 });
    assert.equal(sent.length, 2);
});

test("menu actions share FIFO order, preserve their snapshot and return completion receipts", async () =>
{
    const sent: string[] = [];
    const first = Promise.withResolvers<undefined>();
    const menu = Promise.withResolvers<ViewEventReceipt>();
    const request: ContextMenuRequest = {
        sessionId: 1,
        target: { kind: "view", routeId: 1, revision: 1, itemId: "two" }
    };
    const queue = orderedViewEvents(async value =>
    {
        assert.ok(value.operation.kind === "event");
        sent.push(value.operation.event.kind);
        if (sent.length === 1)
        {
            await first.promise;
        }
        return { viewRevision: 2, navigationRevision: 3 };
    }, async (value, actionId, confirmed) =>
    {
        assert.deepEqual(value, request);
        assert.equal(actionId, "copy");
        assert.equal(confirmed, true);
        sent.push("menu");
        return menu.promise;
    });
    const selection = queue.event(_request({ kind: "selectionChanged", item_id: "two" }));
    const action = queue.menu(request, "copy", true);
    const after = queue.event(_request({ kind: "selectionChanged", item_id: "three" }));
    assert.deepEqual(sent, ["selectionChanged"]);
    first.resolve(undefined);
    await selection;
    assert.deepEqual(sent, ["selectionChanged", "menu"]);
    menu.resolve({ viewRevision: 2, navigationRevision: 5 });
    assert.deepEqual(await action, { viewRevision: 2, navigationRevision: 5 });
    await after;
    assert.deepEqual(sent, ["selectionChanged", "menu", "selectionChanged"]);
});

test("overflow rejects only the new request and preserves all accepted action barriers", async () =>
{
    const first = Promise.withResolvers<undefined>();
    let count = 0;
    const queue = orderedViewEvents(async () =>
    {
        count++;
        if (count === 1)
        {
            await first.promise;
        }
        return { viewRevision: count, navigationRevision: count * 2 };
    }, async () =>
    {
        throw new Error("Unexpected menu invocation");
    });
    const accepted = Array.from({ length: 17 }, async () =>
        queue.event(_request({
            kind: "actionInvoked",
            invocation: "default",
            item_id: "two",
            action_id: "copy"
        })));
    await assert.rejects(
        queue.event(_request({ kind: "actionInvoked", invocation: "default", item_id: "two", action_id: "copy" })),
        /pending/
    );
    first.resolve(undefined);
    await Promise.all(accepted);
    assert.equal(count, 17);
});

function _request(event: ViewEvent): ViewEventRequest
{
    return { sessionId: 1, routeId: 1, revision: 1, operation: { kind: "event", event } };
}
