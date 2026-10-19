import { TextSurface } from "../../src/syntax/TextSurface";

/**
 * Exercise native boundary points across token replacement without serializing source prefixes.
 */
export function verifySelection(): number
{
    const container = document.createElement("div");
    const outside = document.createTextNode("outside");
    const root = document.createElement("div");
    container.append(outside, root);
    container.style.cssText = "position:fixed;left:-10000px;white-space:pre";
    document.body.append(container);
    const surface = new TextSurface(root);
    surface.append(`${"prefix ".repeat(50)}\n${"body ".repeat(60)}\n`);
    surface.append(`${"tail ".repeat(60)}😀\n`);
    const selection = getSelection();
    if (!selection)
    {
        container.remove();
        throw new Error("Native selection is unavailable");
    }
    const stringify = Range.prototype.toString;
    let checked = 0;
    try
    {
        for (const backward of [false, true])
        {
            for (const boundary of ["text", "token", "segment", "root", "outside", "caret"])
            {
                const first = surface.segments[0];
                const last = surface.segments.at(-1);
                if (!first || !last)
                {
                    throw new Error("Missing source segments");
                }
                const tokens = new Uint32Array([0, 12, 2, last.start, last.start + 12, 3]);
                surface.paint(tokens, 0, surface.segments.length, 0, surface.length);
                let anchor: [Node, number] = first.point(5);
                let focus: [Node, number] = last.point(last.start + 10);
                if (boundary === "token")
                {
                    const token = first.element.firstChild;
                    if (!token)
                    {
                        throw new Error("Missing token node");
                    }
                    anchor = [token, 1];
                }
                else if (boundary === "segment")
                {
                    anchor = [first.element, 1];
                    focus = [last.element, last.element.childNodes.length];
                }
                else if (boundary === "root")
                {
                    anchor = [root, 0];
                    focus = [root, root.childNodes.length];
                }
                else if (boundary === "outside")
                {
                    anchor = [outside, 2];
                }
                else if (boundary === "caret")
                {
                    anchor = focus;
                }
                const start = backward ? focus : anchor;
                const end = backward ? anchor : focus;
                selection.setBaseAndExtent(...start, ...end);
                const expected = _state(container, selection);
                // The assertion covers capture during both removal and insertion of token spans.
                Range.prototype.toString = () =>
                {
                    throw new Error("Selection preservation serialized source text");
                };
                surface.paint(null, 0, 0);
                surface.paint(tokens, 0, surface.segments.length, 0, surface.length);
                Range.prototype.toString = stringify;
                if (_state(container, selection) !== expected)
                {
                    throw new Error(`Selection changed across repaint: ${boundary}, backward=${backward}`);
                }
                checked++;
                selection.removeAllRanges();
            }
        }
        return checked;
    }
    finally
    {
        Range.prototype.toString = stringify;
        selection.removeAllRanges();
        surface.clear();
        container.remove();
    }
}

function _state(root: HTMLElement, selection: Selection): string
{
    if (!selection.anchorNode || !selection.focusNode)
    {
        throw new Error("Selection was lost");
    }
    const range = document.createRange();
    range.selectNodeContents(root);
    range.setEnd(selection.anchorNode, selection.anchorOffset);
    const anchor = range.toString().length;
    range.setEnd(selection.focusNode, selection.focusOffset);
    return `${anchor}:${range.toString().length}`;
}
