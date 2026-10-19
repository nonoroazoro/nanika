/**
 * Preserve native selection direction and focus when token spans replace text nodes.
 */
export class TextSelection
{
    private readonly _selection: Selection;
    private readonly _anchor: [Node, number] | number;
    private readonly _focus: [Node, number] | number;
    private readonly _active: Element | null;

    private constructor(selection: Selection, anchor: [Node, number] | number, focus: [Node, number] | number)
    {
        this._selection = selection;
        this._anchor = anchor;
        this._focus = focus;
        this._active = document.activeElement;
    }

    static capture(root: HTMLElement, offset: (node: Node, offset: number) => number): TextSelection | null
    {
        const selection = getSelection();
        if (
            !selection?.anchorNode || !selection.focusNode
            || (!root.contains(selection.anchorNode) && !root.contains(selection.focusNode))
        )
        {
            return null;
        }
        const anchor = _capture(root, selection.anchorNode, selection.anchorOffset, offset);
        const focus = selection.isCollapsed
            ? anchor
            : _capture(root, selection.focusNode, selection.focusOffset, offset);
        return new TextSelection(selection, anchor, focus);
    }

    restore(point: (offset: number) => [Node, number]): void
    {
        const anchor = typeof this._anchor === "number" ? point(this._anchor) : this._anchor;
        const focus = typeof this._focus === "number" ? point(this._focus) : this._focus;
        this._selection.setBaseAndExtent(anchor[0], anchor[1], focus[0], focus[1]);
        if (document.activeElement !== this._active)
        {
            if (this._active === document.body)
            {
                (document.activeElement as HTMLElement | null)?.blur();
            }
            else if (this._active instanceof HTMLElement)
            {
                this._active.focus({ preventScroll: true });
            }
        }
    }
}

function _capture(
    root: HTMLElement,
    node: Node,
    offset: number,
    locate: (node: Node, offset: number) => number
): [Node, number] | number
{
    if (!root.contains(node))
    {
        return [node, offset];
    }
    return locate(node, offset);
}
