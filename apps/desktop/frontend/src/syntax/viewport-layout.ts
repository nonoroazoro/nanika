import { VIEWPORT_METRICS_LIMIT, VIEWPORT_SEGMENT_LIMIT } from "./viewport-limits";

import type { TextSurface } from "./TextSurface";
import type { ViewportContext } from "./ViewportContext";
import type { ViewportLayout } from "./ViewportLayout";

// Token identity ties measurements to immutable source, revision and palette.
// Weak ownership follows the bounded token cache, without retaining source or DOM.
const layouts = new WeakMap<Uint32Array, ViewportLayout>();

/**
 * Capture before appending source to avoid an otherwise redundant plain-text layout.
 *
 * @param element Empty source container with its final source typography
 * @param viewport Consumer-owned scrolling viewport
 */
export function viewportContext(element: HTMLElement, viewport: HTMLElement): ViewportContext
{
    const source = element.getBoundingClientRect();
    const bounds = viewport.getBoundingClientRect();
    const style = getComputedStyle(element);
    return {
        offset: source.top - bounds.top,
        width: bounds.width,
        height: bounds.height,
        metrics: JSON.stringify([
            source.width,
            devicePixelRatio,
            style.font,
            style.fontKerning,
            style.fontVariantLigatures,
            style.fontFeatureSettings,
            style.fontVariationSettings,
            style.textRendering,
            style.letterSpacing,
            style.wordSpacing,
            style.lineHeight,
            style.whiteSpace,
            style.wordBreak,
            style.overflowWrap,
            style.tabSize,
            style.textTransform,
            style.direction
        ])
    };
}

/**
 * Retain one measured viewport per resident token map, never a detached subtree.
 *
 * @param tokens Complete immutable token map
 * @param layout Measured source interval and its exact rendering context
 */
export function rememberViewportLayout(tokens: Uint32Array, layout: ViewportLayout): void
{
    if (layout.context.metrics.length > VIEWPORT_METRICS_LIMIT || layout.heights.length > VIEWPORT_SEGMENT_LIMIT)
    {
        layouts.delete(tokens);
        return;
    }
    layouts.set(tokens, layout);
}

/**
 * Restore validated geometry before the first source layout. A miss uses normal measurement.
 *
 * @param surface Newly populated source surface
 * @param tokens Complete immutable token map
 * @param context Rendering context captured before source was appended
 */
export function restoreViewportLayout(
    surface: TextSurface,
    tokens: Uint32Array,
    context: ViewportContext
): ViewportLayout | null
{
    const layout = layouts.get(tokens);
    if (
        layout?.context.metrics !== context.metrics || layout.context.offset !== context.offset
        || layout.context.width !== context.width || layout.context.height !== context.height
        || layout.length !== surface.length
    )
    {
        return null;
    }
    surface.paint(tokens, layout.first, layout.last, layout.from, layout.to);
    surface.restoreHeights(layout.heights);
    return layout;
}
