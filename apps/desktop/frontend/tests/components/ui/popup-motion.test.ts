import assert from "node:assert/strict";
import { afterEach, beforeEach, test, vi } from "vitest";

import { menuMotion } from "../../../src/components/ui/popup-motion";

const harness = vi.hoisted(() => ({
    state: "open",
    running: false,
    rendered: { transform: "matrix(0.98, 0, 0, 0.98, 0, 0)", opacity: "0.65" },
    between: vi.fn(),
    keyframes: vi.fn(),
    destroy: vi.fn(),
    changed: () =>
    {}
}));
vi.mock("../../../src/components/motion/Motion", () => ({
    Motion: class
    {
        between = harness.between;
        keyframes = harness.keyframes;
        destroy = harness.destroy;

        get running(): boolean
        {
            return harness.running;
        }

        read = (properties: string[]) => (harness.running
            ? Object.fromEntries(
                properties.map(property => [property, harness.rendered[property as keyof typeof harness.rendered]])
            )
            : this.destination(properties));

        destination = (properties: string[]) =>
        {
            const style: Record<string, string> = harness.state === "open"
                ? { transform: "matrix(1, 0, 0, 1, 0, 0)", opacity: "1" }
                : { transform: "matrix(0.96, 0, 0, 0.96, 0, 0)", opacity: "0" };
            return Object.fromEntries(properties.map(property => [property, style[property]]));
        };
    }
}));

beforeEach(() =>
{
    harness.state = "open";
    harness.running = false;
    harness.between.mockReset();
    harness.keyframes.mockReset();
    harness.destroy.mockReset();
    vi.stubGlobal("getComputedStyle", () => ({ getPropertyValue: () => "linear" }));
    vi.stubGlobal("MutationObserver", _observer);
    vi.stubGlobal("DOMMatrixReadOnly", _matrix);
});
afterEach(() => vi.unstubAllGlobals());

test("settled menu dismissal starts visibly, before the closed CSS destination", () =>
{
    const element = { dataset: { state: "open" } } as unknown as HTMLElement;
    const cleanup = menuMotion(element);
    harness.between.mockClear();
    harness.state = "closed";
    element.dataset.state = "closed";
    harness.changed();
    assert.deepEqual(harness.between.mock.calls[0]?.slice(0, 2), [{ opacity: "1" }, { opacity: "0" }]);
    assert.deepEqual(harness.between.mock.calls[1]?.[1], { transform: "scale(0.96)" });
    cleanup?.();
    assert.equal(harness.destroy.mock.calls.length, 2);
});

test("an interrupted entrance dismisses from its rendered opacity and scale", () =>
{
    const element = { dataset: { state: "open" } } as unknown as HTMLElement;
    const cleanup = menuMotion(element);
    harness.between.mockClear();
    harness.running = true;
    harness.state = "closed";
    element.dataset.state = "closed";
    harness.changed();
    assert.deepEqual(harness.between.mock.calls[0]?.[0], { opacity: "0.65" });
    assert.deepEqual(harness.between.mock.calls[1]?.[0], { transform: harness.rendered.transform });
    assert.deepEqual(harness.between.mock.calls[1]?.[1], { transform: "scale(0.94)" });
    const calls = harness.between.mock.calls.length;
    harness.changed();
    assert.equal(harness.between.mock.calls.length, calls);
    cleanup?.();
});

function _observer(callback: () => void)
{
    harness.changed = callback;
    return {
        observe: () =>
        {},
        disconnect: () =>
        {}
    };
}

function _matrix(transform: string)
{
    return { a: Number(transform.match(/matrix\(([^,]+)/)?.[1]) };
}
