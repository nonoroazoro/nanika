<script lang="ts">
import type { DetailView } from "../types";
const { detail }: { detail: DetailView; } = $props();
</script>

<article aria-label={detail.title ?? "Details"}>
    {#if detail.title}<h2>{detail.title}</h2>{/if}
    {#if detail.image_data_url}
        <img
            class="preview"
            src={detail.image_data_url}
            alt={detail.title ?? "Clipboard image"}
            loading="lazy"
            decoding="async"
        />
    {:else}
        <pre class="copyable">{detail.body}</pre>
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
pre { margin: 0; white-space: pre-wrap; overflow-wrap: anywhere; font: inherit; font-size: var(--font-row); user-select: text; }
.copyable { padding: var(--space-3); border: 1px solid var(--border-subtle); border-radius: var(--radius-row); background: var(--surface-raised); color: var(--text-primary); }
h2 { margin: 0 0 var(--space-3); font-size: var(--font-meta); font-weight: 600; overflow-wrap: anywhere; }
.preview { display: block; max-width: 100%; max-height: 55vh; object-fit: contain; border-radius: var(--radius-row); background: var(--surface-raised); }
dl { margin-top: var(--space-6); font-size: var(--font-meta); }
dl div { display: grid; grid-template-columns: 1fr 1fr; gap: var(--space-3); padding: var(--space-2) 0; border-top: 1px solid var(--border-subtle); }
dt { color: var(--text-secondary); }
dd { margin: 0; overflow-wrap: anywhere; }
</style>
