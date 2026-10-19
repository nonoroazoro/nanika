import { rememberViewportLayout, viewportContext } from "./viewport-layout";
import { VIEWPORT_SEGMENT_LIMIT } from "./viewport-limits";

import type { TextSurface } from "./TextSurface";
import type { ViewportLayout } from "./ViewportLayout";

/**
 * Color only visible source segments. The complete text stays available to native selection.
 */
export class ViewportTokens
{
    private _tokens: Uint32Array = new Uint32Array();
    private readonly _resize: ResizeObserver;
    private readonly _surface: TextSurface;
    private readonly _viewport: HTMLElement;
    private _frame = 0;
    private _disposed = false;
    private _geometry: readonly number[] = [];
    private _paintedTokens: Uint32Array | null = null;
    private _metrics: string | null = null;
    private readonly _demand: ((end: number) => void) | null;

    constructor(
        surface: TextSurface,
        viewport: HTMLElement,
        restored: ViewportLayout | null = null,
        demand: ((end: number) => void) | null = null
    )
    {
        this._demand = demand;
        this._surface = surface;
        this._viewport = viewport;
        if (restored)
        {
            this._geometry = restored.geometry;
            this._paintedTokens = restored.tokens;
            this._metrics = restored.context.metrics;
            this._demand?.(restored.to);
        }
        viewport.addEventListener("scroll", this._schedule, { passive: true });
        // ResizeObserver already runs before paint. Another rAF exposes plain rows
        // for a frame in WKWebView; foreground-only spans preserve observed geometry.
        this._resize = new ResizeObserver(() =>
        {
            this.paint();
        });
        this._resize.observe(viewport);
        this._resize.observe(surface.element);
    }

    setTokens(tokens: Uint32Array): void
    {
        this._tokens = tokens;
        this.paint();
    }

    paint(): void
    {
        if (this._disposed)
        {
            return;
        }
        let context = viewportContext(this._surface.element, this._viewport);
        if (this._metrics !== null && this._metrics !== context.metrics)
        {
            this._surface.clearCachedLayout();
            this._paintedTokens = null;
            context = viewportContext(this._surface.element, this._viewport);
        }
        this._metrics = context.metrics;
        const viewport = this._viewport.getBoundingClientRect();
        const source = this._surface.element.getBoundingClientRect();
        const geometry = [
            viewport.top,
            viewport.bottom,
            viewport.width,
            source.top,
            source.height,
            this._surface.length
        ];
        if (this._paintedTokens === this._tokens && geometry.every((value, index) => value === this._geometry[index]))
        {
            return;
        }
        this._geometry = geometry;
        this._paintedTokens = this._tokens;
        const first = this._boundary(viewport.top - 32, false);
        const last = Math.min(this._surface.segments.length, this._boundary(viewport.bottom + 32, true) + 1);
        // One logical line can wrap far beyond the viewport. Bound its token DOM
        // by visual rows as well, while retaining the full original text node tail.
        const from = this._surface.segments[first]?.boundary(viewport.top - 32, false) ?? 0;
        const to = this._surface.segments[last - 1]?.boundary(viewport.bottom + 32, true) ?? 0;
        this._demand?.(to);
        if (this._tokens.length && this._surface.segments.length <= VIEWPORT_SEGMENT_LIMIT)
        {
            rememberViewportLayout(this._tokens, {
                context,
                tokens: this._tokens,
                geometry,
                heights: this._surface.segments.map(segment => segment.element.getBoundingClientRect().height),
                length: this._surface.length,
                first,
                last,
                from,
                to
            });
        }
        this._surface.paint(this._tokens, first, last, from, to);
    }

    dispose(): void
    {
        this._disposed = true;
        cancelAnimationFrame(this._frame);
        this._resize.disconnect();
        this._viewport.removeEventListener("scroll", this._schedule);
        // No observer remains to revoke height hints if hidden or disabled text resizes.
        this._surface.clearCachedLayout();
    }

    private readonly _schedule = (): void =>
    {
        if (!this._disposed && !this._frame)
        {
            this._frame = requestAnimationFrame(() =>
            {
                this._frame = 0;
                this.paint();
            });
        }
    };

    private _boundary(y: number, after: boolean): number
    {
        let low = 0;
        let high = this._surface.segments.length;
        while (low < high)
        {
            const middle = (low + high) >>> 1;
            const segment = this._surface.segments[middle];
            if (!segment)
            {
                break;
            }
            const rect = segment.element.getBoundingClientRect();
            if ((after ? rect.top : rect.bottom) < y)
            {
                low = middle + 1;
            }
            else
            {
                high = middle;
            }
        }
        return low;
    }
}
