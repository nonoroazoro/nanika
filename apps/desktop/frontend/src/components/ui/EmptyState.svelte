<script lang="ts">
import type { Snippet } from "svelte";

const { title, description, icon }: {
    title: string;
    description?: string;
    icon?: Snippet;
} = $props();
</script>

<div class="empty-state" role="status" aria-atomic="true">
    {#if icon}<div class="empty-icon" aria-hidden="true">{@render icon()}</div>{/if}
    <p class="empty-title">{title}</p>
    {#if description}<p class="empty-description">{description}</p>{/if}
</div>

<style>
.empty-state { display: flex; width: 100%; min-width: 0; flex-direction: column; align-items: center; gap: var(--space-2); padding: var(--space-6) var(--space-4); text-align: center; overflow-wrap: anywhere; }
.empty-icon { display: grid; place-items: center; margin-bottom: var(--space-2); color: var(--text-tertiary); }
.empty-title { margin: 0; color: var(--text-primary); font-size: var(--font-row); font-weight: 600; line-height: 1.35; }
.empty-description { margin: 0; max-width: 24rem; color: var(--text-secondary); font-size: var(--font-meta); line-height: 1.5; }
</style>
