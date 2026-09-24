import assert from "node:assert/strict";
import { test, vi } from "vitest";

import { uiActivity } from "../../src/ui/activity.ts";

import type { UiActivity } from "../../src/types/UiActivity.ts";

test("WebView consumers share activity listeners and observe current state after resubscription", () =>
{
    let focused = true;
    const documentEvents = Object.assign(new EventTarget(), {
        visibilityState: "visible",
        hasFocus: () => focused
    });
    const windowEvents = new EventTarget();
    const documentListen = vi.spyOn(documentEvents, "addEventListener");
    const windowListen = vi.spyOn(windowEvents, "addEventListener");
    const documentRemove = vi.spyOn(documentEvents, "removeEventListener");
    const windowRemove = vi.spyOn(windowEvents, "removeEventListener");
    vi.stubGlobal("document", documentEvents);
    vi.stubGlobal("window", windowEvents);
    const first: UiActivity[] = [];
    const second: UiActivity[] = [];
    const stopFirst = uiActivity.subscribe(value =>
    {
        first.push(value);
    });
    const stopSecond = uiActivity.subscribe(value =>
    {
        second.push(value);
    });
    try
    {
        assert.deepEqual(first.at(-1), { visible: true, focused: true });
        assert.equal(documentListen.mock.calls.length, 1);
        assert.equal(windowListen.mock.calls.length, 2);
        focused = false;
        windowEvents.dispatchEvent(new Event("blur"));
        assert.deepEqual(second.at(-1), { visible: true, focused: false });
        stopFirst();
        assert.equal(windowRemove.mock.calls.length, 0);
        documentEvents.visibilityState = "hidden";
        documentEvents.dispatchEvent(new Event("visibilitychange"));
        assert.deepEqual(second.at(-1), { visible: false, focused: false });
        stopSecond();
        assert.equal(documentRemove.mock.calls.length, 1);
        assert.equal(windowRemove.mock.calls.length, 2);
        documentEvents.visibilityState = "visible";
        focused = true;
        const stopResumed = uiActivity.subscribe(value =>
        {
            first.push(value);
        });
        assert.deepEqual(first.at(-1), { visible: true, focused: true });
        stopResumed();
    }
    finally
    {
        stopFirst();
        stopSecond();
        vi.unstubAllGlobals();
        vi.restoreAllMocks();
    }
});
