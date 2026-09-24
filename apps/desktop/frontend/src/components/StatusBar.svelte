<script lang="ts">
import Button from "./Button.svelte";
import ShortcutKeys from "./ShortcutKeys.svelte";

import type { StatusBarEntry } from "../types/StatusBarEntry";

const { leadingEntries = [], trailingEntries = [], onInvoke }: {
    leadingEntries?: StatusBarEntry[];
    trailingEntries?: StatusBarEntry[];
    onInvoke?: (id: string) => void;
} = $props();
const statusAnnouncement = $derived.by(() =>
{
    const titles: string[] = [];
    for (const entry of leadingEntries)
    {
        if (entry.pending?.active || entry.confirmation?.active)
        {
            titles.push(entry.pending?.active ? entry.pending.title : entry.confirmation?.title ?? "");
        }
    }
    for (const entry of trailingEntries)
    {
        if (entry.pending?.active || entry.confirmation?.active)
        {
            titles.push(entry.pending?.active ? entry.pending.title : entry.confirmation?.title ?? "");
        }
    }
    return titles.join(". ");
});

function displayedTitle(entry: StatusBarEntry): string
{
    if (entry.pending?.active)
    {
        return entry.pending.title;
    }
    if (entry.confirmation?.active)
    {
        return entry.confirmation.title;
    }
    return entry.title;
}

function handleInvoke(event: MouseEvent): void
{
    event.stopPropagation();
    const id = event.currentTarget instanceof HTMLElement ? event.currentTarget.dataset.entryId : undefined;
    if (id && onInvoke)
    {
        onInvoke(id);
    }
}
</script>

{#snippet statusEntry(entry: StatusBarEntry)}
    {#snippet content()}
        {#if entry.icon === "app"}
            <svg
                class="app-mark"
                width="20"
                height="20"
                viewBox="0 0 24 24"
                fill="currentColor"
                aria-hidden="true"
            >
                <path d="M5.75 18V6h2.4l7.7 8.25V6h2.4v12h-2.4l-7.7-8.25V18Z" />
            </svg>
        {/if}
        {#if !entry.iconOnly}
            <span class="entry-label-stack" aria-hidden="true">
                <span class="entry-label-reserve">{entry.title}</span>
                {#if entry.confirmation}
                    <span class="entry-label-reserve">{entry.confirmation.title}</span>
                {/if}
                {#if entry.pending}
                    <span class="entry-label-reserve">{entry.pending.title}</span>
                {/if}
                <span class="entry-label">{displayedTitle(entry)}</span>
            </span>
        {/if}
        {#if entry.keys?.length}<ShortcutKeys keys={entry.keys} />{/if}
    {/snippet}
    {#if entry.interactive === false}
        <span
            class="status-entry passive"
            role="group"
            class:pending={Boolean(entry.pending?.active)}
            aria-busy={entry.pending?.active ? "true" : undefined}
            aria-keyshortcuts={entry.ariaShortcut}
            aria-label={displayedTitle(entry)}
        >
            {@render content()}
        </span>
    {:else}
        <Button
            class={["status-entry", {
                "icon-only": entry.iconOnly,
                selected: entry.menu?.expanded,
                pending: Boolean(entry.pending?.active)
            }]}
            variant={entry.destructive ? "danger" : "ghost"}
            data-entry-id={entry.id}
            disabled={entry.disabled || Boolean(entry.pending?.active)}
            aria-busy={entry.pending?.active ? "true" : undefined}
            aria-keyshortcuts={entry.ariaShortcut}
            aria-label={displayedTitle(entry)}
            aria-haspopup={entry.menu ? "menu" : undefined}
            aria-expanded={entry.menu?.expanded}
            aria-controls={entry.menu?.expanded ? entry.menu.controls : undefined}
            onclick={handleInvoke}
        >
            {@render content()}
        </Button>
    {/if}
{/snippet}

{#if leadingEntries.length || trailingEntries.length}
    <span class="status-announcement" role="status" aria-live="polite" aria-atomic="true">
        {statusAnnouncement}
    </span>
    <!-- Preserve editing focus on pointer press; button click actions still run. -->
    <footer
        class="status-bar"
        onmousedowncapture={(event => event.preventDefault())}
    >
        <div class="leading-entries">
            {#each leadingEntries as entry (entry.id)}
                {@render statusEntry(entry)}
            {/each}
        </div>
        <div class="trailing-entries">
            {#each trailingEntries as entry, index (entry.id)}
                {#if index > 0}<span class="separator" aria-hidden="true"></span>{/if}
                {@render statusEntry(entry)}
            {/each}
        </div>
    </footer>
{/if}

<style>
.status-bar { display: flex; flex: 0 0 auto; min-height: 3rem; align-items: center; gap: var(--space-3); padding: 0 var(--space-5); border-top: 1px solid var(--border-subtle); background: var(--surface-window); }
.leading-entries, .trailing-entries { display: flex; min-width: 0; align-items: center; gap: var(--space-2); }
.leading-entries { flex: 1 1 auto; }
.trailing-entries { flex: 0 1 auto; margin-left: auto; }
.status-bar :global(.status-entry) { display: inline-flex; min-width: 0; max-width: 40vw; flex: 0 1 auto; align-items: center; gap: var(--space-2); white-space: nowrap; }
.passive { min-height: var(--control-height); padding: 0.25rem 0.375rem; border: 1px solid transparent; color: var(--text-primary); font-size: var(--font-meta); font-weight: 500; line-height: 1.2; }
.status-bar :global(button.icon-only) { width: var(--icon-size); min-width: var(--icon-size); height: var(--icon-size); min-height: var(--icon-size); flex: 0 0 var(--icon-size); justify-content: center; border: 0; border-radius: 0.625rem; background: transparent; color: var(--text-tertiary); padding: 0; }
.status-bar :global(button.icon-only .app-mark) { opacity: 0.48; transform-origin: center bottom; transform-box: fill-box; transition: opacity var(--motion-control) var(--motion-ease), color var(--motion-control) var(--motion-ease); }
.status-bar :global(button.icon-only:hover:not(:disabled)), .status-bar :global(button.icon-only.selected) { background: transparent; color: var(--text-secondary); }
.status-bar :global(button.icon-only:hover:not(:disabled) .app-mark) { opacity: 0.82; }
.status-bar :global(button.icon-only.selected .app-mark) { opacity: 0.82; }
.status-bar :global(button.icon-only:focus-visible) { background: transparent; box-shadow: none; color: var(--text-secondary); }
.status-bar :global(button.icon-only:focus-visible .app-mark) { opacity: 0.9; }
.status-bar :global(button.icon-only:active:not(:disabled) .app-mark) { opacity: 0.95; transform: none; }
.status-bar :global(button.pending:disabled) { opacity: 1; }
.entry-label-stack { display: grid; min-width: 0; max-width: 40vw; }
.entry-label, .entry-label-reserve { grid-area: 1 / 1; min-width: 0; white-space: nowrap; }
.entry-label { overflow: hidden; text-overflow: ellipsis; }
.entry-label-reserve { visibility: hidden; }
:global(.pending) .entry-label { color: var(--text-secondary); }
@media (prefers-reduced-motion: no-preference) {
  :global(:root[data-ui-active="true"]) .status-bar :global(button.icon-only:hover:not(:disabled) .app-mark) { animation: app-mark-hop 520ms both; }
  :global(:root[data-ui-active="true"]) .status-bar :global(button.icon-only:active:not(:disabled) .app-mark) { transform: translateY(1px) scaleX(1.045) scaleY(0.9); transition-duration: 70ms; animation: none; }
  :global(:root[data-ui-active="true"]) :global(.pending) .entry-label { background: linear-gradient(100deg, var(--text-secondary) 20%, var(--text-primary) 50%, var(--text-secondary) 80%); background-position: 100% 0; background-size: 250% 100%; background-clip: text; color: transparent; -webkit-background-clip: text; -webkit-text-fill-color: transparent; animation: var(--motion-loading); }
}
.separator { width: 1px; height: 1rem; margin: 0 var(--space-1); background: var(--border-subtle); }
.status-announcement { position: absolute; width: 1px; height: 1px; overflow: hidden; clip-path: inset(50%); white-space: nowrap; }

@keyframes -global-status-bar-shimmer {
  0%, 18% { background-position: 100% 0; }
  82%, 100% { background-position: 0 0; }
}
@keyframes app-mark-hop {
  0% { transform: translateY(0) scale(1); animation-timing-function: cubic-bezier(0.3, 0, 0.5, 1); }
  12% { transform: translateY(0) scaleX(1.03) scaleY(0.96); animation-timing-function: cubic-bezier(0.2, 0.8, 0.2, 1); }
  36% { transform: translateY(-2.4px) scaleX(0.985) scaleY(1.02); animation-timing-function: cubic-bezier(0.4, 0, 0.8, 0.2); }
  60% { transform: translateY(0) scaleX(1.025) scaleY(0.975); animation-timing-function: cubic-bezier(0.2, 0.8, 0.2, 1); }
  78% { transform: translateY(-0.8px) scaleX(0.995) scaleY(1.005); animation-timing-function: cubic-bezier(0.4, 0, 0.2, 1); }
  100% { transform: translateY(0) scale(1); }
}
</style>
