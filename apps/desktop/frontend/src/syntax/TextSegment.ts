/**
 * Stable source segment. Only foreground spans change as it enters the viewport.
 */
export class TextSegment
{
    readonly element = document.createElement("span");
    readonly start: number;
    text: string;
    private _tokens: Uint32Array | null = null;
    private _revision = 0;
    private _paintedRevision = 0;
    private _paintedStart = 0;
    private _paintedEnd = 0;

    constructor(start: number, text: string)
    {
        this.start = start;
        this.text = text;
        this.element.className = "syntax-source-segment";
        this.element.textContent = text;
    }

    get end(): number
    {
        return this.start + this.text.length;
    }

    append(text: string): void
    {
        this.text += text;
        this._revision++;
        this.element.append(document.createTextNode(text));
    }

    paint(tokens: Uint32Array | null, from = this.start, to = this.end): void
    {
        if (
            this._tokens === tokens && this._paintedRevision === this._revision
            && (!tokens || (from === this._paintedStart && to === this._paintedEnd))
        )
        {
            return;
        }
        this._tokens = tokens;
        this._paintedRevision = this._revision;
        this._paintedStart = from;
        this._paintedEnd = to;
        if (!tokens)
        {
            this.element.textContent = this.text;
            return;
        }
        const fragment = document.createDocumentFragment();
        let low = 0;
        let high = tokens.length / 3;
        while (low < high)
        {
            const middle = (low + high) >>> 1;
            if ((tokens[(middle * 3) + 1] ?? 0) <= from)
            {
                low = middle + 1;
            }
            else
            {
                high = middle;
            }
        }
        let cursor = this.start;
        for (let index = low * 3; index < tokens.length && (tokens[index] ?? 0) < to; index += 3)
        {
            const start = Math.max(from, tokens[index] ?? 0);
            const end = Math.min(to, tokens[index + 1] ?? 0);
            if (cursor < start)
            {
                fragment.append(document.createTextNode(this.text.slice(cursor - this.start, start - this.start)));
            }
            const token = document.createElement("span");
            token.className = `syntax-token-${tokens[index + 2]}`;
            token.textContent = this.text.slice(start - this.start, end - this.start);
            fragment.append(token);
            cursor = end;
        }
        if (cursor < this.end)
        {
            fragment.append(document.createTextNode(this.text.slice(cursor - this.start)));
        }
        this.element.replaceChildren(fragment);
    }

    /**
     * Locate a visual row boundary without assuming glyph widths or splitting source lines.
     *
     * @param y Viewport-relative vertical coordinate
     * @param after Include the visual row intersecting the boundary
     */
    boundary(y: number, after: boolean): number
    {
        const bounds = this.element.getBoundingClientRect();
        if (y <= bounds.top)
        {
            return this.start;
        }
        if (y >= bounds.bottom)
        {
            return this.end;
        }
        const range = document.createRange();
        let low = 0;
        let high = this.text.length;
        while (low < high)
        {
            const middle = (low + high) >>> 1;
            const start = this.point(this.start + middle);
            const end = this.point(this.start + middle + 1);
            range.setStart(start[0], start[1]);
            range.setEnd(end[0], end[1]);
            const rect = range.getBoundingClientRect();
            if ((after ? rect.top : rect.bottom) < y)
            {
                low = middle + 1;
            }
            else
            {
                high = middle;
            }
        }
        // Keep UTF-16 surrogate pairs intact at a clipped color boundary.
        const previous = this.text.charCodeAt(low - 1);
        const current = this.text.charCodeAt(low);
        if (previous >= 0xd800 && previous <= 0xdbff && current >= 0xdc00 && current <= 0xdfff)
        {
            low += after ? 1 : -1;
        }
        return this.start + low;
    }

    offset(node: Node, offset: number): number
    {
        let length = node.nodeType === Node.TEXT_NODE ? offset : 0;
        if (node.nodeType !== Node.TEXT_NODE)
        {
            let child = node.firstChild;
            for (let index = 0; child && index < offset; index++, child = child.nextSibling)
            {
                length += _textLength(child);
            }
        }
        for (let current: Node | null = node; current && current !== this.element; current = current.parentNode)
        {
            for (let sibling = current.previousSibling; sibling; sibling = sibling.previousSibling)
            {
                length += _textLength(sibling);
            }
        }
        return this.start + length;
    }

    point(offset: number): [Node, number]
    {
        let remaining = offset - this.start;
        const walker = document.createTreeWalker(this.element, NodeFilter.SHOW_TEXT);
        for (let node = walker.nextNode(); node; node = walker.nextNode())
        {
            const length = (node as Text).length;
            if (remaining <= length)
            {
                return [node, remaining];
            }
            remaining -= length;
        }
        return [this.element, this.element.childNodes.length];
    }
}

function _textLength(node: Node): number
{
    if (node.nodeType === Node.TEXT_NODE)
    {
        return (node as Text).length;
    }
    let length = 0;
    const walker = document.createTreeWalker(node, NodeFilter.SHOW_TEXT);
    for (let text = walker.nextNode(); text; text = walker.nextNode())
    {
        length += (text as Text).length;
    }
    return length;
}
