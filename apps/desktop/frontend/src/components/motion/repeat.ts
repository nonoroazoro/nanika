import { animate } from "animejs/animation";

import type { AnimationParams } from "animejs";
import type { Attachment } from "svelte/attachments";

import { motionAvailability } from "./policy";
import { duration } from "./tokens";

/**
 * Own a visual loop only while its mounted surface can animate.
 * @param parameters Existing keyframes and timing
 * @param token CSS duration token
 */
export function repeatMotion(parameters: AnimationParams, token: string): Attachment<HTMLElement>
{
    return element =>
    {
        let stop: (() => void) | undefined;
        const unsubscribe = motionAvailability().subscribe(enabled =>
        {
            stop?.();
            stop = undefined;
            if (enabled)
            {
                const animation = animate(element, { ...parameters, duration: duration(element, token), loop: true });
                stop = () =>
                {
                    animation.revert();
                };
            }
        });
        return () =>
        {
            unsubscribe();
            stop?.();
        };
    };
}
