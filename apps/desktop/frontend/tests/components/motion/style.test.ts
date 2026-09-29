import assert from "node:assert/strict";
import { afterEach, beforeEach, test, vi } from "vitest";

import { styleMotion } from "../../../src/components/motion/style";

const harness = vi.hoisted(() => ({
    color: "rest",
    rendered: null as string | null,
    running: false,
    observations: [] as Array<{ options?: MutationObserverInit; update: () => void; }>,
    between: vi.fn(),
    destroy: vi.fn()
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
        read = () => ({ color: harness.rendered ?? harness.color });
        destination = () => ({ color: harness.color });
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
