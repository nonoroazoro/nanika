/**
 * Shared prefetch distance for both scope replacement and continuation loading.
 * @param viewportHeight Visible height in CSS pixels
 */
export function listPrefetchDistance(viewportHeight: number): number
{
    return Math.max(520, viewportHeight * 2);
}

/**
 * Size a new result prefix beyond the continuation boundary so it is published once.
 * @param viewportHeight Visible height in CSS pixels
 * @param rowHeight Measured collection row height in CSS pixels
 */
export function initialListItemCount(viewportHeight: number, rowHeight: number): number
{
    // Include a complete row beyond the observer edge, including fractional CSS pixels.
    return Math.ceil((viewportHeight + listPrefetchDistance(viewportHeight)) / rowHeight) + 1;
}
