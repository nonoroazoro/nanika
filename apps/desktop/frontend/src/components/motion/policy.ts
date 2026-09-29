import { derived, readable } from "svelte/store";

import type { Readable } from "svelte/store";

import { uiActivity } from "../../ui/activity";

const reduced = readable(false, set =>
{
    const media = window.matchMedia("(prefers-reduced-motion: reduce)");
    const update = (): void =>
    {
        set(media.matches);
    };
    update();
    media.addEventListener("change", update);
    return () =>
    {
        media.removeEventListener("change", update);
    };
});

// Primitive stores suppress repeated activity notifications with identical policy.
const activeMotion = derived(
    [uiActivity, reduced],
    ([activity, reduce]) => activity.visible && activity.focused && !reduce
);
const visibleMotion = derived([uiActivity, reduced], ([activity, reduce]) => activity.visible && !reduce);

/**
 * Shared live availability. Inactive surfaces settle instead of resuming stale motion.
 * @param requireFocus False only for native caption feedback on a visible window
 */
export function motionAvailability(requireFocus = true): Readable<boolean>
{
    return requireFocus ? activeMotion : visibleMotion;
}

/**
 * Native theme changes update CSS states even without a pointer or attribute event.
 */
export const motionAppearance = readable(false, set =>
{
    const media = window.matchMedia("(prefers-color-scheme: dark)");
    const update = (): void =>
    {
        set(media.matches);
    };
    update();
    media.addEventListener("change", update);
    return () =>
    {
        media.removeEventListener("change", update);
    };
});
