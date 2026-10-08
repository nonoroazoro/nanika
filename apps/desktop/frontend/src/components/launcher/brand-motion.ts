import type { Attachment } from "svelte/attachments";

import { Motion } from "../motion/Motion";

import type { MotionKeyframe } from "../motion/MotionKeyframe";

/**
 * Unfold the suffix from N and retract it in reverse without changing text layout.
 * One bounded timeline keeps every glyph inside the menu's exit lifetime.
 */
export const brandMotion: Attachment<HTMLElement> = element =>
{
    const menu = element.closest(".context-menu");
    if (!menu)
    {
        return;
    }
    const glyphs = Array.from(element.querySelectorAll<HTMLElement>(".brand-glyph"), glyph =>
    {
        const shape = new Motion(glyph);
        const fade = new Motion(glyph);
        return { shape, fade, restShape: shape.read(["transform"]), restFade: fade.read(["opacity"]) };
    });
    let open = menu.getAttribute("data-state") === "open";
    const transition = (initial: boolean): void =>
    {
        const ease = getComputedStyle(element).getPropertyValue("--motion-menu-expand-ease").trim();
        const token = open ? "--motion-brand-enter" : "--motion-menu-exit";
        glyphs.forEach((glyph, index) =>
        {
            let fromShape = glyph.shape.running ? glyph.shape.read(["transform"]) : glyph.restShape;
            let fromFade = glyph.fade.running ? glyph.fade.read(["opacity"]) : glyph.restFade;
            if (initial)
            {
                fromShape = { transform: "translate(-4px, 2px)" };
                fromFade = { opacity: "0" };
            }
            glyph.restShape = glyph.shape.destination(["transform"]);
            glyph.restFade = glyph.fade.destination(["opacity"]);
            const alpha = Number(fromFade.opacity);
            // A partly revealed letter reverses immediately instead of waiting for its stagger again.
            const partial = alpha > 0 && alpha < 1;
            const rank = open ? index : glyphs.length - 1 - index;
            const start = partial ? 0 : 0.6 * rank / Math.max(1, glyphs.length - 1);
            glyph.shape.keyframes(
                "transform",
                _frames(fromShape.transform, glyph.restShape.transform, start, ease),
                token
            );
            glyph.fade.keyframes("opacity", _frames(fromFade.opacity, glyph.restFade.opacity, start, ease), token);
        });
    };
    if (open)
    {
        transition(true);
    }
    const observer = new MutationObserver(() =>
    {
        const next = menu.getAttribute("data-state") === "open";
        if (next !== open)
        {
            open = next;
            transition(false);
        }
    });
    observer.observe(menu, { attributes: true, attributeFilter: ["data-state"] });
    return () =>
    {
        observer.disconnect();
        glyphs.forEach(({ shape, fade }) =>
        {
            shape.destroy();
            fade.destroy();
        });
    };
};

function _frames(
    from: string,
    to: string,
    start: number,
    ease: string
): [MotionKeyframe, MotionKeyframe, ...MotionKeyframe[]]
{
    return [
        { value: from, offset: 0, easing: "linear" },
        { value: from, offset: start, easing: ease },
        { value: to, offset: Math.min(1, start + 0.4), easing: "linear" },
        { value: to, offset: 1, easing: "linear" }
    ];
}
