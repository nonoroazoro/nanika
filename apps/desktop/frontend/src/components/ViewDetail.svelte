<script lang="ts">
import type { DetailView } from "../types";
import CachedFileIcon from "./CachedFileIcon.svelte";
const COLLECTION_PREVIEW_LIMIT = 3;
const FILE_PATH_PREVIEW_LIMIT = 5;
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

<article class:image-detail={detail.content.kind === "image"} aria-label={detail.title ?? "Details"}>
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
        {#if detail.content.files.length === 1}
            <div class="file-preview">
                <CachedFileIcon
                    reference={detail.content.files[0]?.icon ?? null}
                    {resourceOrigin}
                    {extensionId}
                    preview
                />
            </div>
            <ul class="files" aria-label="Files">
                <li>
                    <span class="file-icon"><CachedFileIcon
                            reference={detail.content.files[0]?.icon ?? null}
                            {resourceOrigin}
                            {extensionId}
                        /></span>
                    <span>{detail.content.files[0]?.name}</span>
                </li>
            </ul>
        {:else}
            <div
                class="collection-preview"
                role="img"
                aria-label={`${detail.content.files.length} item collection`}
            >
                <div class="collection-stack" aria-hidden="true">
                    {#each detail.content.files.slice(0, COLLECTION_PREVIEW_LIMIT) as file, index (`${index}:${file.path}`)}
                        <span>
                            <CachedFileIcon
                                reference={file.icon}
                                {resourceOrigin}
                                {extensionId}
                                collection
                            />
                        </span>
                    {/each}
                </div>
            </div>
            <ul class="files" aria-label="File collection">
                <li>
                    <span class="file-icon"><CachedFileIcon
                            reference={detail.content.files[0]?.icon ?? null}
                            {resourceOrigin}
                            {extensionId}
                        /></span>
                    <span>{detail.content.files.length} items</span>
                </li>
            </ul>
        {/if}
    {/if}
    {#if detail.metadata.length || detail.content.kind === "files"}
        <dl>
            {#if detail.content.kind === "files"}
                <div>
                    <dt>{detail.content.files.length === 1 ? "Path" : "Paths"}</dt>
                    <dd class="file-paths">
                        {#each detail.content.files.slice(0, FILE_PATH_PREVIEW_LIMIT) as file, index (`${index}:${file.path}`)}
                            <span>{file.path}</span>
                        {/each}
                        {#if detail.content.files.length > FILE_PATH_PREVIEW_LIMIT}
                            <span class="remaining-files">{detail.content.files.length - FILE_PATH_PREVIEW_LIMIT} more {
                                    detail.content.files.length - FILE_PATH_PREVIEW_LIMIT === 1 ? "file" : "files"
                                }</span>
                        {/if}
                    </dd>
                </div>
            {/if}
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
article { min-width: 0; padding: var(--space-5); }
article.image-detail { display: flex; flex-direction: column; height: 100%; }
.image-detail > h2, .image-detail > dl { flex: 0 0 auto; }
article > :last-child { margin-bottom: 0; }
.copyable { display: block; width: 100%; margin: 0; overflow: hidden; resize: none; appearance: none; padding: 0; border: 0; border-radius: 0; background: transparent; color: var(--text-primary); font: inherit; font-size: var(--font-row); line-height: 1.45; }
h2 { margin: 0 0 var(--space-3); font-size: var(--font-meta); font-weight: 600; overflow-wrap: anywhere; }
.image-preview { position: relative; flex: 1 1 auto; width: 100%; min-height: 12rem; max-height: 55vh; overflow: hidden; border-radius: var(--radius-row); background: var(--surface-raised); }
.image-preview img { position: absolute; inset: 0; display: block; width: 100%; height: 100%; object-fit: scale-down; }
.files { display: grid; gap: var(--space-1); margin: 0; padding: 0; list-style: none; }
.files li { display: grid; grid-template-columns: 2rem minmax(0, 1fr); align-items: center; gap: var(--space-3); min-height: var(--row-height); padding: var(--space-2) 0; border-bottom: 1px solid var(--border-subtle); }
.files li:last-child { border-bottom: 0; }
.files + dl { margin-top: 0; }
.file-preview { margin-bottom: var(--space-3); }
.collection-preview { display: grid; min-height: 9rem; place-items: center; margin-bottom: var(--space-3); overflow: hidden; }
.collection-stack { display: flex; isolation: isolate; width: min(100%, 15rem); height: 7rem; align-items: center; justify-content: center; }
.collection-stack > span { display: grid; place-items: center; flex: 0 0 5.5rem; height: 5.5rem; min-width: 0; margin: 0 -0.5rem; }
.collection-stack > span:first-child { transform: translateY(0.25rem) rotate(-4deg); }
.collection-stack > span:last-child { transform: translateY(0.25rem) rotate(4deg); }
.collection-stack > span:nth-child(1) { z-index: 0; }
.collection-stack > span:nth-child(2) { z-index: 1; }
.collection-stack > span:nth-child(3) { z-index: 2; }
.file-icon { position: relative; display: grid; width: var(--icon-size); height: var(--icon-size); place-items: center; }
.files span { min-width: 0; overflow-wrap: anywhere; }
dl { margin-top: var(--space-6); font-size: var(--font-meta); }
dl div { display: grid; grid-template-columns: 6rem minmax(0, 1fr); gap: var(--space-3); padding: var(--space-2) 0; border-top: 1px solid var(--border-subtle); }
dt { color: var(--text-secondary); }
dd { min-width: 0; margin: 0; overflow-wrap: anywhere; }
.file-paths { display: grid; gap: var(--space-2); user-select: text; }
.file-paths span { display: block; overflow-wrap: anywhere; white-space: pre-wrap; }
.remaining-files { color: var(--text-secondary); }
</style>
