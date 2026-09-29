// Keep the physical track below native layout limits. Rows retain their measured height;
// only empty distance is compressed when the complete collection exceeds this extent.
const MAX_SCROLL_EXTENT = 4_000_000;

/**
 * Map a complete collection onto a representable native scroll track.
 * @param totalHeight Full logical height including section headings
 * @param viewportHeight Visible native viewport height
 */
export function scrollGeometry(totalHeight: number, viewportHeight: number)
{
    const extent = Math.min(totalHeight, MAX_SCROLL_EXTENT);
    const logicalRange = Math.max(0, totalHeight - viewportHeight);
    const physicalRange = Math.max(0, extent - viewportHeight);
    return {
        extent,
        logical(top: number): number
        {
            return _scale(top, physicalRange, logicalRange);
        },
        physical(top: number): number
        {
            return _scale(top, logicalRange, physicalRange);
        }
    };
}

function _scale(value: number, source: number, target: number): number
{
    const bounded = Math.max(0, Math.min(source, value));
    if (source === 0 || source === target)
    {
        return bounded;
    }
    // Divide first so each exact endpoint maps to its exact destination endpoint.
    return (bounded / source) * target;
}
