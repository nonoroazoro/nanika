/**
 * Compare every visible colored source interval with the complete reference token map.
 * Run outside timing samples because geometry inspection is deliberately exhaustive.
 *
 * @param source Native source container
 * @param viewport Scrolling viewport
 * @param tokens Complete start/end/palette triples
 */
export function verifyViewportColors(source: HTMLElement, viewport: HTMLElement, tokens: Uint32Array)
{
    const bounds = viewport.getBoundingClientRect();
    const walker = document.createTreeWalker(source, NodeFilter.SHOW_TEXT);
    const range = document.createRange();
    let offset = 0;
    let tokenIndex = 0;
    let checked = 0;
    let mismatches = 0;
    const failures: Array<
        { actual: number; bottom: number; end: number; expected: number; start: number; top: number; }
    > = [];
    for (let node = walker.nextNode(); node; node = walker.nextNode())
    {
        const end = offset + (node as Text).length;
        while (tokenIndex < tokens.length && (tokens[tokenIndex + 1] ?? 0) <= offset)
        {
            tokenIndex += 3;
        }
        const colored = node.parentElement?.closest("[class^=syntax-token-]");
        const actual = Number(colored?.className.slice("syntax-token-".length)) || 1;
        for (let index = tokenIndex; index < tokens.length && (tokens[index] ?? 0) < end; index += 3)
        {
            range.setStart(node, Math.max(offset, tokens[index] ?? 0) - offset);
            range.setEnd(node, Math.min(end, tokens[index + 1] ?? 0) - offset);
            if (
                [...range.getClientRects()].some(rect =>
                {
                    return rect.height > 0 && rect.width > 0 && rect.top < bounds.bottom && rect.bottom > bounds.top
                        && rect.left < bounds.right && rect.right > bounds.left;
                })
            )
            {
                checked++;
                if (actual !== tokens[index + 2])
                {
                    mismatches++;
                    if (failures.length < 8)
                    {
                        const rect = range.getBoundingClientRect();
                        failures.push({
                            start: Math.max(offset, tokens[index] ?? 0),
                            end: Math.min(end, tokens[index + 1] ?? 0),
                            actual,
                            expected: tokens[index + 2] ?? 0,
                            top: rect.top,
                            bottom: rect.bottom
                        });
                    }
                }
            }
        }
        offset = end;
    }
    return { checked, mismatches, failures };
}
