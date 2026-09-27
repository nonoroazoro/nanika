<script lang="ts">
import Button from "./Button.svelte";
import type { ViewPagination } from "../types";
const { pagination, busy, label, onPage }: {
    pagination: ViewPagination;
    busy: boolean;
    label: string;
    onPage: (cursor: string) => void;
} = $props();
</script>

<nav aria-label={label}>
    <Button
        disabled={busy || pagination.previous_cursor === null}
        onclick={() =>
        {
            if (pagination.previous_cursor !== null)
            {
                onPage(pagination.previous_cursor);
            }
        }}
    >Previous</Button>
    <span aria-live="polite">{pagination.label}</span>
    <Button
        disabled={busy || pagination.next_cursor === null}
        onclick={() =>
        {
            if (pagination.next_cursor !== null)
            {
                onPage(pagination.next_cursor);
            }
        }}
    >Next</Button>
</nav>

<style>
nav { display: flex; align-items: center; justify-content: space-between; gap: var(--space-2); padding: var(--space-2); border-top: 1px solid var(--border-subtle); }
span { color: var(--text-secondary); font-size: var(--font-meta); text-align: center; }
</style>
