import type { ViewportContext } from "./ViewportContext";

/**
 * Numeric viewport measurements only. No DOM node survives a document switch.
 */
export interface ViewportLayout
{
    context: ViewportContext;
    tokens: Uint32Array;
    geometry: readonly number[];
    heights: readonly number[];
    length: number;
    first: number;
    last: number;
    from: number;
    to: number;
}
