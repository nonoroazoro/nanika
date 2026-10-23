import type { HighlightPart } from "../types/HighlightPart";

const segmenter = new Intl.Segmenter(undefined, { granularity: "grapheme" });
// Single-line rows truncate long titles. Bound decorative work without truncating their content.
const maximumHighlightUnits = 1024;

/**
 * Highlight literal query terms in a bounded prefix of the displayed title. Alias and pinyin matches
 * do not supply offsets into this text. Grapheme mapping preserves original content.
 *
 * @param title Currently displayed text
 * @param query Accepted search query
 */
export function highlightTitle(title: string, query: string): HighlightPart[]
{
    const terms = query.normalize("NFC").replace(/[\u2066-\u2069]/gu, "").match(/[\p{L}\p{N}\p{M}]+/gu);
    if (!terms?.length)
    {
        return [{ text: title, highlighted: false }];
    }
    const graphemes: Array<{ normalized: string; text: string; }> = [];
    let prefixEnd = 0;
    for (const { segment, index } of segmenter.segment(title))
    {
        if (index >= maximumHighlightUnits)
        {
            break;
        }
        // Ignore bidi isolates for comparison while retaining every original display unit.
        graphemes.push({ text: segment, normalized: segment.normalize("NFC").replace(/[\u2066-\u2069]/gu, "") });
        prefixEnd = index + segment.length;
    }
    const normalized = graphemes.map(part => part.normalized).join("");
    const pattern = new RegExp([...new Set(terms)].sort((a, b) => b.length - a.length).join("|"), "giu");
    const matches = [...normalized.matchAll(pattern)];
    const parts: HighlightPart[] = [];
    let offset = 0;
    let matchIndex = 0;
    for (const grapheme of graphemes)
    {
        let match = matches[matchIndex];
        while (match && match.index + match[0].length <= offset)
        {
            match = matches[++matchIndex];
        }
        const highlighted = !!match && match.index < offset + grapheme.normalized.length;
        const previous = parts.at(-1);
        if (previous?.highlighted === highlighted)
        {
            previous.text += grapheme.text;
        }
        else
        {
            parts.push({ text: grapheme.text, highlighted });
        }
        offset += grapheme.normalized.length;
    }
    if (prefixEnd < title.length)
    {
        parts.push({ text: title.slice(prefixEnd), highlighted: false });
    }
    return parts;
}
