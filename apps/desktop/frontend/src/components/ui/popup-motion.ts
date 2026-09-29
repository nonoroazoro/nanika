import type { Attachment } from "svelte/attachments";

import { Motion } from "../motion/Motion";

/**
 * Menus share a scale/fade entrance: 100ms to a 0.3% overshoot, then 50ms to rest.
 * Select popovers retain their displacement feedback. Bits owns presence and focus.
 */
export const menuMotion = _popupMotion(true);
export const popupMotion = _popupMotion(false);

function _popupMotion(menu: boolean): Attachment<HTMLElement>
{
    return element =>
    {
        const fade = new Motion(element);
        const shape = new Motion(element);
        let open = element.dataset.state === "open";
        let restShape = shape.read(["transform"]);
        let restFade = fade.read(["opacity"]);
        const transition = (initial: boolean): void =>
        {
            const style = getComputedStyle(element);
            let from = shape.running ? shape.read(["transform"]) : restShape;
            let alpha = fade.running ? fade.read(["opacity"]) : restFade;
            if (initial)
            {
                from = { transform: menu ? "scale(0.94)" : "translateY(4px)" };
                alpha = { opacity: "0" };
            }
            const destination = shape.destination(["transform"]);
            const opacity = fade.destination(["opacity"]);
            // The state attribute already changed. With no live effect, computed
            // style is the new destination, not the last visible frame.
            restShape = destination;
            restFade = opacity;
            if (menu)
            {
                const ease = style.getPropertyValue(open ? "--motion-menu-expand-ease" : "--motion-menu-exit-ease")
                    .trim();
                fade.between(alpha, opacity, open ? "--motion-menu-enter" : "--motion-menu-exit", ease);
                if (open)
                {
                    shape.keyframes("transform", [
                        { value: from.transform, offset: 0, easing: ease },
                        {
                            value: "scale(1.003)",
                            offset: 2 / 3,
                            easing: style.getPropertyValue("--motion-menu-settle-ease").trim()
                        },
                        { value: destination.transform, offset: 1, easing: "linear" }
                    ], "--motion-menu-enter");
                }
                else
                {
                    // Dismiss from the visible scale, including an interrupted entrance.
                    const scale = new DOMMatrixReadOnly(from.transform).a;
                    shape.between(
                        from,
                        { transform: `scale(${Math.max(0.9, scale - 0.04)})` },
                        "--motion-menu-exit",
                        ease
                    );
                }
            }
            else
            {
                fade.between(alpha, opacity, open ? "--motion-enter" : "--motion-exit");
                shape.between(
                    from,
                    destination,
                    open ? "--motion-popup-enter" : "--motion-exit",
                    open ? style.getPropertyValue("--motion-popup-ease").trim() : undefined
                );
            }
        };
        if (open)
        {
            transition(true);
        }
        const observer = new MutationObserver(() =>
        {
            const next = element.dataset.state === "open";
            if (open !== next)
            {
                open = next;
                transition(false);
            }
        });
        observer.observe(element, { attributes: true, attributeFilter: ["data-state"] });
        return () =>
        {
            observer.disconnect();
            fade.destroy();
            shape.destroy();
        };
    };
}
