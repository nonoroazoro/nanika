import { remove } from "animejs/utils";
import { waapi } from "animejs/waapi";

import type { WAAPIAnimation } from "animejs/waapi";

import { motionAvailability } from "./policy";
import { duration, easing } from "./tokens";

import type { MotionKeyframe } from "./MotionKeyframe";

/**
 * One owner per animated property. Destination styles remain authoritative;
 * Anime.js only supplies the temporary interpolation above them.
 */
export class Motion
{
    private _element: HTMLElement | SVGElement;
    private _animation: WAAPIAnimation | undefined;
    private _enabled = false;
    private _unsubscribe: () => void;

    constructor(element: HTMLElement | SVGElement, requireFocus = true)
    {
        this._element = element;
        this._unsubscribe = motionAvailability(requireFocus).subscribe(enabled =>
        {
            this._enabled = enabled;
            if (!this._enabled)
            {
                this.settle();
            }
        });
    }

    read<T extends string>(properties: T[]): Record<T, string>
    {
        const style = getComputedStyle(this._element);
        return Object.fromEntries(properties.map(property => [property, style.getPropertyValue(property)])) as Record<
            T,
            string
        >;
    }

    get running(): boolean
    {
        return this._animation !== undefined;
    }

    /**
     * Read CSS without our effect, synchronously restoring it before paint.
     * @param properties Owned CSS properties
     */
    destination<T extends string>(properties: T[]): Record<T, string>
    {
        const animations = this._animation?.animations ?? [];
        const effects = animations.map(animation => animation.effect);
        try
        {
            animations.forEach(animation =>
            {
                animation.effect = null;
            });
            return this.read(properties);
        }
        finally
        {
            animations.forEach((animation, index) =>
            {
                animation.effect = effects[index] ?? null;
            });
        }
    }

    /**
     * Retarget from the rendered frame, never from the previous destination.
     * @param next Authoritative inline destination
     * @param token Duration token
     * @param snap Settle immediately after resize or first placement
     */
    to(next: Record<string, string>, token: string, snap = false): void
    {
        const unchanged = Object.entries(next).every(([property, value]) =>
        {
            return this._element.style.getPropertyValue(property) === value;
        });
        if (!snap && unchanged)
        {
            return;
        }
        const from = this.read(Object.keys(next));
        this.settle();
        for (const [property, value] of Object.entries(next))
        {
            this._element.style.setProperty(property, value);
        }
        if (!snap)
        {
            this.between(from, next, token);
        }
    }

    /**
     * Interpolate CSS-owned states and restore their ownership on completion.
     * @param from Rendered starting values
     * @param to CSS or inline destination values
     * @param token Duration token
     * @param ease Optional control-specific curve
     */
    between(from: Record<string, string>, to: Record<string, string>, token: string, ease?: string): void
    {
        this.settle();
        if (!this._enabled)
        {
            return;
        }
        const properties = Object.fromEntries(
            Object.entries(to)
                .filter(([property, value]) => from[property] !== value)
                .map(([property, value]) => [
                    property.replace(/-([a-z])/g, (_, letter: string) => letter.toUpperCase()),
                    [from[property], value]
                ])
        );
        if (!Object.keys(properties).length)
        {
            return;
        }
        this._animate(properties, token, ease);
    }

    /**
     * Interpolate one property through timed samples. The caller supplies the
     * rendered first sample on interruption; Anime.js retains lifecycle ownership.
     * @param property CSS property in JavaScript notation
     * @param frames At least two samples, including the CSS-owned destination
     * @param token Total duration token
     */
    keyframes(property: string, frames: [MotionKeyframe, MotionKeyframe, ...MotionKeyframe[]], token: string): void
    {
        const first = frames[0];
        this.settle();
        if (!this._enabled || frames.every(frame => frame.value === first.value))
        {
            return;
        }
        this._animate({ [property]: frames.map(frame => frame.value) }, token, "linear");
        const effect = this._animation?.animations[0]?.effect as KeyframeEffect | null | undefined;
        effect?.setKeyframes(frames.map(frame => ({
            [property]: frame.value,
            offset: frame.offset,
            easing: frame.easing
        })));
    }

    settle(): void
    {
        const animation = this._animation;
        this._animation = undefined;
        if (animation)
        {
            // Unregister before restoring CSS ownership. Native cancel events are
            // deferred; leaving Anime's lookup alive lets them commit stale styles
            // after revert, freezing later pseudo-state feedback.
            remove(this._element, animation);
            animation.revert();
        }
    }

    destroy(): void
    {
        this._unsubscribe();
        this.settle();
    }

    private _animate(properties: Record<string, unknown>, token: string, ease?: string): void
    {
        this._animation = waapi.animate(this._element, {
            ...properties,
            duration: duration(this._element, token),
            ease: ease ?? easing(this._element),
            onComplete: animation =>
            {
                if (this._animation === animation)
                {
                    this.settle();
                }
            }
        });
    }
}
