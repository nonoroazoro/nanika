import type { Attachment } from "svelte/attachments";

import { uiActivity } from "../../ui/activity";
import { styleMotion } from "../motion/style";

const feedback = styleMotion({
    properties: ["transform", "opacity"],
    scope: "button",
    duration:
        element => (element.closest("button")?.hasAttribute("data-hovered") ? "--motion-popup-enter" : "--motion-exit")
});

/**
 * Only real pointer movement engages the brand. Menu dismissal can expose the
 * button under a stationary pointer; that must not replay its hover entrance.
 */
export const appMarkMotion: Attachment<HTMLElement> = element =>
{
    const button = element.closest("button");
    if (!button)
    {
        return;
    }
    let open = button.getAttribute("aria-expanded") === "true";
    let pointer: { x: number; y: number; } | undefined;
    const move = (event: PointerEvent): void =>
    {
        const changed = pointer?.x !== event.clientX || pointer?.y !== event.clientY;
        pointer = { x: event.clientX, y: event.clientY };
        if (changed && !open && event.pointerType !== "touch")
        {
            button.toggleAttribute("data-hovered", true);
        }
    };
    const leave = (): void =>
    {
        if (!open)
        {
            button.removeAttribute("data-hovered");
        }
    };
    const observer = new MutationObserver(() =>
    {
        const next = button.getAttribute("aria-expanded") === "true";
        if (open && !next)
        {
            button.removeAttribute("data-hovered");
        }
        open = next;
    });
    observer.observe(button, { attributes: true, attributeFilter: ["aria-expanded"] });
    button.addEventListener("pointermove", move);
    button.addEventListener("pointerleave", leave);
    const unsubscribe = uiActivity.subscribe(activity =>
    {
        if (!activity.visible || !activity.focused)
        {
            button.removeAttribute("data-hovered");
            pointer = undefined;
        }
    });
    const cleanup = feedback(element);
    return () =>
    {
        cleanup?.();
        unsubscribe();
        observer.disconnect();
        button.removeEventListener("pointermove", move);
        button.removeEventListener("pointerleave", leave);
        button.removeAttribute("data-hovered");
    };
};
