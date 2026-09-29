import { listPrefetchDistance } from "./list-loading";

import type { ViewSection } from "../types/ViewSection";

/**
 * Map native scroll geometry to a bounded entry range, retaining section headings.
 * @param sections Ordered section metadata and delivered item windows
 * @param top Native viewport scroll position
 * @param height Native viewport height
 * @param rowHeight Measured shared collection row height
 * @param headingHeight Measured section heading height
 */
export function collectionWindow(
    sections: readonly ViewSection[],
    top: number,
    height: number,
    rowHeight: number,
    headingHeight: number
)
{
    const total = sections.reduce((sum, section) => sum + section.total, 0);
    const distance = listPrefetchDistance(height);
    const offset = _indexAt(sections, Math.max(0, top - distance), rowHeight, headingHeight);
    const end = _indexAt(sections, top + height + distance, rowHeight, headingHeight) + 1;
    const count = Math.max(1, Math.min(500, total - offset, end - offset));
    // Refill before the visible window reaches the cached edge, avoiding requests per pixel.
    const requiredStart = _indexAt(sections, Math.max(0, top - (height / 2)), rowHeight, headingHeight);
    const requiredEnd = Math.min(total, _indexAt(sections, top + (height * 1.5), rowHeight, headingHeight) + 1);
    let base = 0;
    let covered = true;
    for (const section of sections)
    {
        const first = Math.max(requiredStart, base);
        const last = Math.min(requiredEnd, base + section.total);
        if (first < last && (first < base + section.offset || last > base + section.offset + section.items.length))
        {
            covered = false;
        }
        base += section.total;
    }
    return { offset, count, covered, total };
}

/**
 * Locate a delivered row by absolute collection index without scanning unloaded records.
 */
export function itemAt(sections: readonly ViewSection[], index: number)
{
    let base = 0;
    for (const section of sections)
    {
        const local = index - base - section.offset;
        if (local >= 0 && local < section.items.length)
        {
            return section.items[local];
        }
        base += section.total;
    }
    return undefined;
}

/**
 * Resolve a delivered identity's absolute position.
 */
export function itemIndex(sections: readonly ViewSection[], id: string): number | null
{
    let base = 0;
    for (const section of sections)
    {
        const index = section.items.findIndex(item => item.id === id);
        if (index >= 0)
        {
            return base + section.offset + index;
        }
        base += section.total;
    }
    return null;
}

/**
 * Locate a row in logical scroll geometry, including every preceding section heading.
 */
export function itemTop(
    sections: readonly ViewSection[],
    index: number,
    rowHeight: number,
    headingHeight: number
): number
{
    let base = 0;
    let top = 0;
    for (const section of sections)
    {
        top += section.title ? headingHeight : 0;
        if (index < base + section.total)
        {
            return top + ((index - base) * rowHeight);
        }
        top += section.total * rowHeight;
        base += section.total;
    }
    return top;
}

function _indexAt(sections: readonly ViewSection[], pixel: number, rowHeight: number, headingHeight: number): number
{
    let base = 0;
    let remaining = pixel;
    for (const section of sections)
    {
        if (section.title)
        {
            remaining = Math.max(0, remaining - headingHeight);
        }
        const height = section.total * rowHeight;
        if (remaining < height)
        {
            return base + Math.floor(remaining / rowHeight);
        }
        remaining -= height;
        base += section.total;
    }
    return Math.max(0, base - 1);
}
