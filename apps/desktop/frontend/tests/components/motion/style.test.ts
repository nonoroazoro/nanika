import assert from "node:assert/strict";
import { afterEach, beforeEach, test, vi } from "vitest";

import { styleMotion } from "../../../src/components/motion/style";

import type { UiActivity } from "../../../src/types/UiActivity";

const harness = vi.hoisted(() => ({
    color: "rest",
    rendered: null as string | null,
    running: false,
    observations: [] as Array<{ options?: MutationObserverInit; update: () => void; }>,
    between: vi.fn(),
    destroy: vi.fn(),
    settle: vi.fn(),
    activity: null as ((activity: UiActivity) => void) | null
}));
vi.mock("../../../src/components/motion/Motion", () => ({
    Motion: class
    {
        get running(): boolean
        {
            return harness.running;
        }

        between = harness.between;
        destroy = harness.destroy;
        settle = harness.settle;
        read = () => ({ color: harness.rendered ?? harness.color });
        destination = () => ({ color: harness.color });
    }
}));
vi.mock("../../../src/ui/activity", () => ({
    uiActivity: {
        subscribe: (update: (activity: UiActivity) => void) =>
        {
            harness.activity = update;
            update({ visible: true, focused: true });
            return () =>
            {
                harness.activity = null;
            };
        }
    }
}));
vi.mock("../../../src/components/motion/policy", () => ({
    motionAppearance: {
        subscribe: () => () =>
        {}
    }
}));

beforeEach(() =>
{
    vi.useFakeTimers();
    harness.color = "rest";
    harness.rendered = null;
    harness.running = false;
    harness.observations = [];
    harness.between.mockReset();
    harness.destroy.mockReset();
    harness.settle.mockReset().mockImplementation(() =>
    {
        harness.running = false;
        harness.rendered = null;
    });
    vi.stubGlobal("KeyboardEvent", Event);
    vi.stubGlobal("MutationObserver", _observer);
});
afterEach(() =>
{
    vi.useRealTimers();
    vi.unstubAllGlobals();
});

test("keyboard feedback reads the native default state after event dispatch without preventing input", () =>
{
    const element = Object.assign(new EventTarget(), { closest: () => null }) as unknown as HTMLElement;
    const cleanup = styleMotion({ properties: ["color"] })(element);
    const press = Object.assign(new Event("keydown", { cancelable: true }), { key: " " });
    element.dispatchEvent(press);
    assert.equal(press.defaultPrevented, false);
    assert.equal(harness.between.mock.calls.length, 0);
    harness.color = "pressed";
    vi.runAllTimers();
    assert.deepEqual(harness.between.mock.calls[0]?.slice(0, 2), [{ color: "rest" }, { color: "pressed" }]);
    element.dispatchEvent(Object.assign(new Event("keyup"), { key: " " }));
    harness.color = "rest";
    vi.runAllTimers();
    assert.deepEqual(harness.between.mock.calls[1]?.slice(0, 2), [{ color: "pressed" }, { color: "rest" }]);
    cleanup?.();
});

test("mounted entry retargets from its rendered frame when interrupted and never replays on unchanged events", async () =>
{
    const element = Object.assign(new EventTarget(), { closest: () => null }) as unknown as HTMLElement;
    const cleanup = styleMotion({
        properties: ["color"],
        enterFrom: { color: "hidden" },
        duration: "--motion-popup-enter"
    })(element);
    assert.deepEqual(harness.between.mock.calls[0], [
        { color: "hidden" },
        { color: "rest" },
        "--motion-popup-enter",
        undefined
    ]);
    harness.running = true;
    harness.rendered = "mid-entry";
    harness.color = "closed";
    element.dispatchEvent(new Event("pointerleave"));
    await Promise.resolve();
    assert.deepEqual(harness.between.mock.calls[1]?.slice(0, 2), [{ color: "mid-entry" }, { color: "closed" }]);
    element.dispatchEvent(new Event("pointerleave"));
    await Promise.resolve();
    assert.equal(harness.between.mock.calls.length, 2);
    cleanup?.();
});

test("unmount cancels queued keyboard work and releases listeners", () =>
{
    const element = Object.assign(new EventTarget(), { closest: () => null }) as unknown as HTMLElement;
    const cleanup = styleMotion({ properties: ["color"] })(element);
    element.dispatchEvent(Object.assign(new Event("keydown"), { key: " " }));
    assert.equal(vi.getTimerCount(), 1);
    cleanup?.();
    assert.equal(vi.getTimerCount(), 0);
    element.dispatchEvent(Object.assign(new Event("keyup"), { key: " " }));
    vi.runAllTimers();
    assert.equal(harness.between.mock.calls.length, 0);
    assert.equal(harness.destroy.mock.calls.length, 1);
    assert.equal(harness.activity, null);
});

test("window activation settles interrupted feedback and refreshes colors before further input", async () =>
{
    const element = Object.assign(new EventTarget(), { closest: () => null }) as unknown as HTMLElement;
    const cleanup = styleMotion({ properties: ["color"], requireFocus: false })(element);
    harness.running = true;
    harness.rendered = "mid-hover";
    harness.color = "inactive";
    harness.activity?.({ visible: true, focused: false });
    assert.equal(harness.running, false);
    assert.equal(harness.between.mock.calls.length, 0);
    element.dispatchEvent(new Event("pointerleave"));
    await Promise.resolve();
    assert.equal(harness.between.mock.calls.length, 0);

    harness.color = "rest";
    harness.activity?.({ visible: true, focused: true });
    harness.color = "hover";
    element.dispatchEvent(new Event("pointerenter"));
    await Promise.resolve();
    assert.deepEqual(harness.between.mock.calls[0]?.slice(0, 2), [{ color: "rest" }, { color: "hover" }]);
    const settled = harness.settle.mock.calls.length;
    harness.activity?.({ visible: true, focused: true });
    assert.equal(harness.settle.mock.calls.length, settled);
    cleanup?.();
});

test("ARIA control state changes animate immediately without replaying on the next pointer event", async () =>
{
    for (const attribute of ["aria-pressed", "aria-selected", "aria-checked"])
    {
        harness.color = "rest";
        harness.between.mockClear();
        const element = Object.assign(new EventTarget(), { closest: () => null }) as unknown as HTMLElement;
        const cleanup = styleMotion({ properties: ["color"] })(element);
        harness.color = "selected";
        const observer = harness.observations.at(-1);
        assert.ok(observer);
        assert.ok(observer.options?.attributeFilter?.includes(attribute));
        observer.update();
        assert.deepEqual(harness.between.mock.calls[0]?.slice(0, 2), [{ color: "rest" }, { color: "selected" }]);
        element.dispatchEvent(new Event("pointerleave"));
        await Promise.resolve();
        assert.equal(harness.between.mock.calls.length, 1);
        cleanup?.();
    }
});

function _observer(update: () => void)
{
    const observation: { options?: MutationObserverInit; update: () => void; } = { update };
    harness.observations.push(observation);
    return {
        observe: (_target: Element, options: MutationObserverInit) =>
        {
            observation.options = options;
        },
        disconnect: vi.fn()
    };
}
