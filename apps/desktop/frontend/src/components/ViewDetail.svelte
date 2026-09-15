<script lang="ts">
import type { DetailView } from "../types";
import SemanticContentIcon from "./SemanticContentIcon.svelte";
const { detail, resourceOrigin, extensionId }: {
    detail: DetailView;
    resourceOrigin: string;
    extensionId: string;
} = $props();

function resolveImageSource(): string | null
{
    if (detail.content.kind !== "image")
    {
        return null;
    }
    return detail.content.source.kind === "dataUrl"
        ? detail.content.source.value
        : `${resourceOrigin}/${extensionId}/${detail.content.source.path}`;
}

const text = $derived(detail.content.kind === "text" ? detail.content.value : null);
const imageSource = $derived(resolveImageSource());
let body = $state<HTMLTextAreaElement>();

$effect(() =>
{
    const value = text;
    if (!body || value === null)
    {
        return;
    }
    body.style.height = "0";
    body.style.height = `${body.scrollHeight}px`;
});
</script>

<article aria-label={detail.title ?? "Details"}>
    {#if detail.title}<h2>{detail.title}</h2>{/if}
    {#if detail.content.kind === "image" && imageSource}
        <div class="image-preview">
            <img
                src={imageSource}
                alt={detail.content.alternative_text}
                loading="lazy"
                decoding="async"
            />
        </div>
    {:else if detail.content.kind === "text"}
        <textarea
            bind:this={body}
            class="copyable"
            value={detail.content.value}
            aria-label="Detail content"
            rows="1"
            readonly
            spellcheck="false"
        ></textarea>
    {:else if detail.content.kind === "files"}
        <ul class="files" aria-label="Files">
            {#each detail.content.names as name, index (`${index}:${name}`)}
                <li>
                    <span class="file-icon"><SemanticContentIcon kind="files" /></span>
                    <span>{name}</span>
                </li>
            {/each}
        </ul>
    {/if}
    {#if detail.metadata.length}
        <dl>
            {#each detail.metadata as entry, index (index)}
                <div>
                    <dt>{entry.title}</dt>
                    <dd>{entry.value}</dd>
                </div>
            {/each}
        </dl>
    {/if}
</article>

<style>
article { min-width: 0; height: 100%; overflow: auto; padding: var(--space-5); }
.copyable { display: block; width: 100%; margin: 0; overflow: hidden; resize: none; appearance: none; padding: 0; border: 0; border-radius: 0; background: transparent; color: var(--text-primary); font: inherit; font-size: var(--font-row); line-height: 1.45; }
h2 { margin: 0 0 var(--space-3); font-size: var(--font-meta); font-weight: 600; overflow-wrap: anywhere; }
.image-preview { display: grid; width: 100%; min-height: 12rem; max-height: 55vh; place-items: center; overflow: hidden; border-radius: var(--radius-row); background: var(--surface-raised); }
.image-preview img { display: block; max-width: 100%; max-height: 55vh; object-fit: contain; }
.files { display: grid; gap: var(--space-1); margin: 0; padding: 0; list-style: none; }
.files li { display: grid; grid-template-columns: 2rem minmax(0, 1fr); align-items: center; gap: var(--space-3); min-height: var(--row-height); padding: var(--space-2) 0; border-bottom: 1px solid var(--border-subtle); }
.file-icon { display: grid; width: var(--icon-size); height: var(--icon-size); place-items: center; }
.files span { min-width: 0; overflow-wrap: anywhere; }
dl { margin-top: var(--space-6); font-size: var(--font-meta); }
dl div { display: grid; grid-template-columns: 1fr 1fr; gap: var(--space-3); padding: var(--space-2) 0; border-top: 1px solid var(--border-subtle); }
dt { color: var(--text-secondary); }
dd { margin: 0; overflow-wrap: anywhere; }
</style>
