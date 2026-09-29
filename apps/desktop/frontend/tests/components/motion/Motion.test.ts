import assert from "node:assert/strict";
import { afterEach, beforeEach, test, vi } from "vitest";

import { Motion } from "../../../src/components/motion/Motion.ts";

import type { MotionKeyframe } from "../../../src/components/motion/MotionKeyframe";

const harness = vi.hoisted(() => ({
    listeners: new Set<(enabled: boolean) => void>(),
    animate: vi.fn(),
    remove: vi.fn()
}));
vi.mock("../../../src/components/motion/policy", () => ({
    motionAvailability: () => ({
        subscribe: (listener: (enabled: boolean) => void) =>
        {
            harness.listeners.add(listener);
            listener(true);
            return () =>
            {
                harness.listeners.delete(listener);
            };
        }
    })
}));
vi.mock("animejs/utils", () => ({ remove: harness.remove }));
vi.mock("animejs/waapi", () => ({ waapi: { animate: harness.animate } }));

beforeEach(() =>
{
    harness.animate.mockReset();
    harness.remove.mockReset();
    harness.listeners.clear();
});
afterEach(() => vi.unstubAllGlobals());

test("first placement and resize snap without an origin flight", () =>
{
    const fixture = _fixture();
    const motion = new Motion(fixture.element);
    motion.to({ transform: "translateY(80px)" }, "--motion-selection", true);
    assert.equal(harness.animate.mock.calls.length, 0);
    assert.equal(fixture.inline.get("transform"), "translateY(80px)");
    motion.destroy();
});

test("retargeting preserves the rendered origin and authoritative inline destination", () =>
{
    const fixture = _fixture();
    const motion = new Motion(fixture.element);
    motion.to({ transform: "translateY(100px)" }, "--motion-selection");
    const first = fixture.animations[0];
    assert(first);
    fixture.rendered.set("transform", "translateY(37px)");
    motion.to({ transform: "translateY(0px)" }, "--motion-selection");
    const second = fixture.animations[1];
    assert(second);
    assert.deepEqual(second.parameters.transform, ["translateY(37px)", "translateY(0px)"]);
    assert.equal(fixture.inline.get("transform"), "translateY(0px)");
    // A late callback from a superseded animation cannot settle its replacement.
    first.complete();
    assert.equal(motion.running, true);
    second.complete();
    assert.equal(motion.running, false);
    assert.equal(fixture.inline.get("transform"), "translateY(0px)");
    motion.destroy();
});

test("repeating a destination does not restart its in-flight animation", () =>
{
    const fixture = _fixture();
    const motion = new Motion(fixture.element);
    motion.to({ opacity: "1" }, "--motion-enter");
    fixture.rendered.set("opacity", "0.5");
    motion.to({ opacity: "1" }, "--motion-enter");
    assert.equal(fixture.animations.length, 1);
    assert.equal(motion.running, true);
    motion.destroy();
});

test("CSS properties use WAAPI keyframe names and regain CSS ownership on completion", () =>
{
    const fixture = _fixture();
    const motion = new Motion(fixture.element);
    motion.between(
        { "background-color": "rgb(0, 0, 0)" },
        { "background-color": "rgb(255, 255, 255)" },
        "--motion-control"
    );
    const animation = fixture.animations[0];
    assert(animation);
    assert.deepEqual(animation.parameters.backgroundColor, ["rgb(0, 0, 0)", "rgb(255, 255, 255)"]);
    animation.complete();
    assert.equal(fixture.element.style.getPropertyValue("background-color"), "");
    motion.destroy();
});

test("interruption unregisters deferred cancellation before restoring CSS ownership", () =>
{
    const fixture = _fixture();
    const motion = new Motion(fixture.element);
    for (let index = 0; index < 100; index++)
    {
        motion.between({ opacity: "0.48" }, { opacity: "0.9" }, "--motion-control");
        motion.settle();
        assert.equal(fixture.inline.get("opacity"), undefined);
        fixture.flushCancelEvents();
        assert.equal(fixture.inline.get("opacity"), undefined);
    }
    motion.between({ opacity: "0.48" }, { opacity: "0.9" }, "--motion-control");
    assert.equal(motion.running, true);
    assert.equal(fixture.animations.length, 101);
    motion.destroy();
    fixture.flushCancelEvents();
    assert.equal(fixture.inline.get("opacity"), undefined);
});

test("unavailable motion settles and releases its subscription", () =>
{
    const fixture = _fixture();
    const motion = new Motion(fixture.element);
    motion.to({ opacity: "1" }, "--motion-enter");
    harness.listeners.forEach(listener =>
    {
        listener(false);
    });
    assert.equal(motion.running, false);
    assert.equal(fixture.inline.get("opacity"), "1");
    motion.to({ opacity: "0" }, "--motion-enter");
    assert.equal(fixture.animations.length, 1);
    assert.equal(fixture.inline.get("opacity"), "0");
    motion.destroy();
    assert.equal(harness.listeners.size, 0);
});

test("timed overshoot runs even when its first and last samples match", () =>
{
    const fixture = _fixture();
    const motion = new Motion(fixture.element);
    const frames = [
        { value: "scale(1)", offset: 0, easing: "linear" },
        { value: "scale(1.003)", offset: 2 / 3, easing: "ease-in-out" },
        { value: "scale(1)", offset: 1, easing: "linear" }
    ] satisfies [MotionKeyframe, MotionKeyframe, MotionKeyframe];
    motion.keyframes("transform", frames, "--motion-menu-enter");
    const animation = fixture.animations[0];
    assert(animation);
    assert.deepEqual(
        animation.frames(),
        frames.map(({ value, offset, easing }) => ({ transform: value, offset, easing }))
    );
    animation.complete();
    assert.equal(motion.running, false);
    motion.destroy();
});

function _fixture()
{
    const inline = new Map<string, string>();
    const rendered = new Map<string, string>();
    const cancelEvents: Array<() => void> = [];
    const animations: Array<{ complete: () => void; frames: () => unknown; parameters: Record<string, unknown>; }> = [];
    // Only the DOM boundary is substituted; interpolation and presence are covered
    // by the browser fixture against real Anime.js and Bits UI.
    const element = {
        style: {
            getPropertyValue: (property: string) => inline.get(property) ?? "",
            setProperty: (property: string, value: string) => inline.set(property, value)
        }
    } as unknown as HTMLElement;
    vi.stubGlobal("getComputedStyle", () => ({
        getPropertyValue: (property: string) =>
        {
            if (property === "--motion-ease")
            {
                return "cubic-bezier(0.2, 0, 0, 1)";
            }
            if (property.startsWith("--motion-"))
            {
                return "220ms";
            }
            return rendered.get(property) ?? inline.get(property) ?? "0";
        }
    }));
    harness.animate.mockImplementation((_element: HTMLElement, parameters: Record<string, unknown>) =>
    {
        const saved = new Map(inline);
        let registered = true;
        let keyframes: unknown;
        const animation = {
            animations: [{
                effect: {
                    setKeyframes: (value: unknown) =>
                    {
                        keyframes = value;
                    }
                }
            }],
            unregister: () =>
            {
                registered = false;
            },
            revert: () =>
            {
                inline.clear();
                saved.forEach((value, key) =>
                {
                    inline.set(key, value);
                });
                rendered.clear();
                // Anime's registry commits styles when the queued cancel event
                // arrives, unless the animation was synchronously unregistered.
                cancelEvents.push(() =>
                {
                    if (registered)
                    {
                        inline.set("opacity", "0.9");
                    }
                });
            }
        };
        animations.push({
            parameters,
            frames: () => keyframes,
            complete: () =>
            {
                (parameters.onComplete as (value: unknown) => void)(animation);
            }
        });
        return animation;
    });
    harness.remove.mockImplementation((_element: HTMLElement, animation: { unregister: () => void; }) =>
    {
        assert.equal(_element, element);
        animation.unregister();
    });
    const flushCancelEvents = (): void =>
    {
        cancelEvents.splice(0).forEach(callback =>
        {
            callback();
        });
    };
    return { element, inline, rendered, animations, flushCancelEvents };
}
