import { TextSegment } from "./TextSegment";
import { TextSelection } from "./TextSelection";

/**
 * Exact native source with independently bounded transport, text segments and token DOM.
 */
export class TextSurface
{
    readonly element: HTMLElement;
    readonly segments: TextSegment[] = [];
    private readonly _segmentByElement = new WeakMap<Node, TextSegment>();
    private _length = 0;
    private _first = 0;
    private _last = 0;
    private _from = 0;
    private _to = 0;
    private _tokens: Uint32Array | null = null;
    private _contained = false;

    constructor(element: HTMLElement)
    {
        this.element = element;
    }

    get length(): number
    {
        return this._length;
    }

    append(text: string): void
    {
        this.clearCachedLayout();
        const fragment = document.createDocumentFragment();
        let start = 0;
        // Keep source blocks on logical-line boundaries. Appending a transport chunk
        // must extend its unfinished last line rather than insert a visual line break.
        const previous = this.segments.at(-1);
        if (previous && !previous.text.endsWith("\n"))
        {
            const newline = text.indexOf("\n");
            start = newline === -1 ? text.length : newline + 1;
            previous.append(text.slice(0, start));
            this._length += start;
            this._tokens = null;
        }
        while (start < text.length)
        {
            const newline = text.indexOf("\n", start + 256);
            const end = newline === -1 ? text.length : newline + 1;
            const segment = new TextSegment(this._length, text.slice(start, end));
            this.segments.push(segment);
            this._segmentByElement.set(segment.element, segment);
            fragment.append(segment.element);
            this._length += end - start;
            start = end;
        }
        this.element.append(fragment);
    }

    paint(tokens: Uint32Array | null, first: number, last: number, from = 0, to = 0): void
    {
        if (
            tokens === this._tokens && first === this._first && last === this._last
            && from === this._from && to === this._to
        )
        {
            return;
        }
        const selection = TextSelection.capture(this.element, (node, offset) => this._offset(node, offset));
        for (let index = this._first; index < this._last; index++)
        {
            if (index < first || index >= last)
            {
                this.segments[index]?.paint(null);
            }
        }
        for (let index = first; index < last; index++)
        {
            const segment = this.segments[index];
            segment?.paint(tokens, Math.max(segment.start, from), Math.min(segment.end, to));
        }
        this._first = first;
        this._last = last;
        this._from = from;
        this._to = to;
        this._tokens = tokens;
        selection?.restore(offset => this._point(offset));
    }

    clear(): void
    {
        this.element.replaceChildren();
        this.segments.length = 0;
        this._length = 0;
        this._first = 0;
        this._last = 0;
        this._from = 0;
        this._to = 0;
        this._tokens = null;
        this._contained = false;
    }

    restoreHeights(heights: readonly number[]): void
    {
        if (heights.length !== this.segments.length || !CSS.supports("content-visibility", "auto"))
        {
            return;
        }
        for (let index = 0; index < this.segments.length; index++)
        {
            const segment = this.segments[index];
            const height = heights[index];
            if (segment && height !== undefined)
            {
                // Exact previously measured heights preserve scroll geometry while
                // the browser skips offscreen glyph layout. Original text stays in DOM.
                segment.element.style.contentVisibility = "auto";
                segment.element.style.containIntrinsicBlockSize = `${height}px`;
            }
        }
        this._contained = true;
    }

    clearCachedLayout(): void
    {
        if (!this._contained)
        {
            return;
        }
        for (const segment of this.segments)
        {
            segment.element.style.removeProperty("content-visibility");
            segment.element.style.removeProperty("contain-intrinsic-block-size");
        }
        this._contained = false;
    }

    private _offset(node: Node, offset: number): number
    {
        if (node === this.element)
        {
            return this.segments[offset]?.start ?? this._length;
        }
        let element = node;
        while (element.parentNode && element.parentNode !== this.element)
        {
            element = element.parentNode;
        }
        const segment = this._segmentByElement.get(element);
        if (!segment)
        {
            throw new Error("Selection does not belong to a source segment");
        }
        return segment.offset(node, offset);
    }

    private _point(offset: number): [Node, number]
    {
        let low = 0;
        let high = this.segments.length;
        while (low < high)
        {
            const middle = (low + high) >>> 1;
            if ((this.segments[middle]?.end ?? 0) < offset)
            {
                low = middle + 1;
            }
            else
            {
                high = middle;
            }
        }
        const segment = this.segments[low];
        if (!segment)
        {
            return [this.element, this.element.childNodes.length];
        }
        return segment.point(offset);
    }
}
