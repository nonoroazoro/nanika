<script lang="ts">
import { untrack } from "svelte";
import LoadMoreBoundary from "../ui/LoadMoreBoundary.svelte";
import type { DetailContent } from "../../types/DetailContent";

const { content, viewport, busy, revision, onRead }: {
    content: Extract<DetailContent, { kind: "text"; }>;
    viewport: HTMLElement | null;
    busy: boolean;
    revision: number;
    onRead: (textId: string, index: number) => Promise<number | null>;
} = $props();
let element = $state<HTMLDivElement>();
let next = $state(0);
let identity: string | null = null;
let textNode: Text | null = null;

$effect(() =>
{
    const target = element;
    const chunk = content;
    if (!target)
    {
        return;
    }
    untrack(() =>
    {
        if (identity !== chunk.text_id || textNode?.parentNode !== target)
        {
            identity = chunk.text_id;
            textNode = document.createTextNode("");
            target.replaceChildren(textNode);
            next = 0;
        }
        // Selection/status snapshots may repeat a chunk. Only the next contiguous chunk appends.
        // appendData preserves existing DOM Range endpoints; replacing textContent does not.
        if (chunk.chunk_index === next && textNode)
        {
            textNode.appendData(chunk.value);
            next++;
        }
    });
});

function _preventEdit(event: Event): void
{
    // A native editing host supplies caret movement and scoped Select All, but the document is read-only.
    event.preventDefault();
}
</script>

<div
    bind:this={element}
    class="text-document"
    role="textbox"
    aria-label="Detail content"
    aria-readonly="true"
    aria-multiline="true"
    tabindex="0"
    contenteditable="true"
    spellcheck="false"
    onbeforeinput={_preventEdit}
    onpaste={_preventEdit}
    oncut={_preventEdit}
    ondrop={_preventEdit}
>
</div>
<LoadMoreBoundary
    {viewport}
    {busy}
    {revision}
    scope={content.text_id}
    cursor={next < content.total_chunks ? String(next) : null}
    onLoad={index => onRead(content.text_id, Number(index))}
/>

<style>
.text-document { min-width: 0; white-space: pre-wrap; overflow-wrap: anywhere; color: var(--text-primary); font: inherit; font-size: var(--font-row); line-height: 1.45; -webkit-user-select: text; user-select: text; }
</style>
