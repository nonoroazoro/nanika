import type { Attachment } from "svelte/attachments";

import { Motion } from "./Motion";
import { motionAppearance } from "./policy";
import { uiActivity } from "../../ui/activity";

import type { StyleMotionOptions } from "./StyleMotionOptions";

/**
 * Animate only declared visual properties. CSS retains pseudo-class and Bits
 * state styling; this attachment never changes input or component state.
 * @param options Owned properties and their local state source
 */
export function styleMotion(options: StyleMotionOptions): Attachment<HTMLElement | SVGElement>
{
    return element =>
    {
        const scope = options.scope ? element.closest(options.scope) ?? element : element;
        const motion = new Motion(element, options.requireFocus);
        let destination = motion.read(options.properties);
        let alive = true;
        let active: boolean | undefined;
        const unsubscribeActivity = uiActivity.subscribe(activity =>
        {
            const next = activity.visible && activity.focused;
            if (next !== active)
            {
                active = next;
                // main.ts publishes the root activity attribute before controls observe it.
                // Snap native activation changes and discard cached colors before the next hover.
                motion.settle();
                destination = motion.read(options.properties);
            }
        });
        const duration = (): string =>
        {
            return typeof options.duration === "function"
                ? options.duration(element)
                : options.duration ?? "--motion-control";
        };
        if (options.enterFrom)
        {
            motion.between(options.enterFrom, destination, duration(), options.ease);
        }
        const update = (): void =>
        {
            if (!alive)
            {
                return;
            }
            // Sample before removing the effect, then read the underlying CSS state.
            const rendered = motion.read(options.properties);
            const running = motion.running;
            const next = motion.destination(options.properties);
            if (options.properties.every(property => next[property] === destination[property]))
            {
                return;
            }
            const from = running ? rendered : destination;
            destination = next;
            motion.between(
                from,
                next,
                duration(),
                options.ease
            );
        };
        const events = [
            "pointerenter",
            "pointerleave",
            "pointerdown",
            "pointerup",
            "pointercancel",
            "focusin",
            "focusout"
        ];
        // Microtasks run after the native pseudo-state and Bits state have updated.
        const schedule = (): void =>
        {
            queueMicrotask(update);
        };
        for (const event of events)
        {
            scope.addEventListener(event, schedule);
        }
        let keyboardTask: ReturnType<typeof setTimeout> | undefined;
        const keyboard = (event: Event): void =>
        {
            if (!(event instanceof KeyboardEvent) || (event.key !== " " && event.key !== "Enter"))
            {
                return;
            }
            // Native keyboard activation updates :active after event microtasks.
            // Observe it in the next task without intercepting the default action.
            clearTimeout(keyboardTask);
            keyboardTask = setTimeout(update, 0);
        };
        scope.addEventListener("keydown", keyboard);
        scope.addEventListener("keyup", keyboard);
        const observer = new MutationObserver(update);
        observer.observe(scope, {
            attributes: true,
            attributeFilter: [
                "class",
                "disabled",
                "data-state",
                "data-hovered",
                "data-revealed",
                "aria-disabled",
                "aria-busy",
                "aria-expanded",
                "aria-pressed",
                "aria-selected",
                "aria-checked"
            ]
        });
        const fieldset = element.closest("fieldset");
        if (fieldset && fieldset !== scope)
        {
            observer.observe(fieldset, { attributes: true, attributeFilter: ["disabled"] });
        }
        const unsubscribeAppearance = motionAppearance.subscribe(update);
        return () =>
        {
            alive = false;
            clearTimeout(keyboardTask);
            scope.removeEventListener("keydown", keyboard);
            scope.removeEventListener("keyup", keyboard);
            unsubscribeAppearance();
            unsubscribeActivity();
            observer.disconnect();
            for (const event of events)
            {
                scope.removeEventListener(event, schedule);
            }
            motion.destroy();
        };
    };
}

export const buttonMotion = styleMotion({ properties: ["background-color", "border-color", "color"] });
