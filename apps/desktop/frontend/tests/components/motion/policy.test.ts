import assert from "node:assert/strict";
import { afterEach, beforeEach, test, vi } from "vitest";

import type { Writable } from "svelte/store";

import { motionAvailability } from "../../../src/components/motion/policy";
import { uiActivity } from "../../../src/ui/activity";

import type { UiActivity } from "../../../src/types/UiActivity";

vi.mock("../../../src/ui/activity", async () =>
{
    const { writable } = await import("svelte/store");
    return { uiActivity: writable({ visible: true, focused: true }) };
});
const activity = uiActivity as Writable<UiActivity>;
let reduced = false;
const listeners = new Set<() => void>();

beforeEach(() =>
{
    reduced = false;
    activity.set({ visible: true, focused: true });
    vi.stubGlobal("window", {
        matchMedia: () => ({
            get matches()
            {
                return reduced;
            },
            addEventListener: (_event: string, listener: () => void) => listeners.add(listener),
            removeEventListener: (_event: string, listener: () => void) => listeners.delete(listener)
        })
    });
});
afterEach(() =>
{
    assert.equal(listeners.size, 0);
    vi.unstubAllGlobals();
});

test("repeated activity events do not restart allowed motion", () =>
{
    const values: boolean[] = [];
    const unsubscribe = motionAvailability().subscribe(value =>
    {
        values.push(value);
    });
    activity.set({ visible: true, focused: true });
    activity.set({ visible: true, focused: true });
    assert.deepEqual(values, [true]);
    activity.set({ visible: true, focused: false });
    activity.set({ visible: false, focused: false });
    assert.deepEqual(values, [true, false]);
    activity.set({ visible: true, focused: true });
    assert.deepEqual(values, [true, false, true]);
    unsubscribe();
});

test("caption feedback ignores focus but honors visibility and live reduced motion", () =>
{
    const content: boolean[] = [];
    const caption: boolean[] = [];
    const stopContent = motionAvailability().subscribe(value =>
    {
        content.push(value);
    });
    const stopCaption = motionAvailability(false).subscribe(value =>
    {
        caption.push(value);
    });
    assert.equal(listeners.size, 1);
    activity.set({ visible: true, focused: false });
    assert.deepEqual(content, [true, false]);
    assert.deepEqual(caption, [true]);
    reduced = true;
    listeners.forEach(listener =>
    {
        listener();
    });
    assert.deepEqual(caption, [true, false]);
    reduced = false;
    listeners.forEach(listener =>
    {
        listener();
    });
    assert.deepEqual(caption, [true, false, true]);
    activity.set({ visible: false, focused: false });
    assert.deepEqual(caption, [true, false, true, false]);
    stopContent();
    assert.equal(listeners.size, 1);
    stopCaption();
});
