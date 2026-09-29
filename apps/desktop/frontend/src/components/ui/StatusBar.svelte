<script lang="ts">
import { cubicBezier } from "animejs/easings/cubic-bezier";
import { repeatMotion } from "../motion/repeat";
import AppMark from "../icons/AppMark.svelte";
import { appMarkMotion } from "./app-mark-motion";
import Button from "./Button.svelte";
import ShortcutKeys from "./ShortcutKeys.svelte";

import type { StatusBarEntry } from "../../types/StatusBarEntry";

const shimmer = repeatMotion({
    keyframes: {
        "0%": { backgroundPosition: "100% 0px" },
        "18%": { backgroundPosition: "100% 0px", ease: cubicBezier(0.42, 0, 0.58, 1) },
        "82%": { backgroundPosition: "0% 0px" },
        "100%": { backgroundPosition: "0% 0px" }
    }
}, "--motion-progress-duration");

const { leadingEntries = [], trailingEntries = [], onInvoke }: {
    leadingEntries?: StatusBarEntry[];
    trailingEntries?: StatusBarEntry[];
    onInvoke?: (id: string, trigger: HTMLButtonElement) => void;
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
    const trigger = event.currentTarget;
    if (trigger instanceof HTMLButtonElement && trigger.dataset.entryId && onInvoke)
    {
        onInvoke(trigger.dataset.entryId, trigger);
    }
}
</script>

{#snippet statusEntry(entry: StatusBarEntry)}
    {#snippet content()}
        {#if entry.icon === "app"}
            <span class="app-mark-motion" {@attach appMarkMotion}><AppMark /></span>
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
                <span class="entry-label" {@attach entry.pending?.active ? shimmer : undefined}>{
                    displayedTitle(entry)
                }</span>
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
            feedback={entry.icon !== "app"}
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
/* The owning window supplies the surface; attached menus follow their trigger. */
.status-bar { display: flex; flex: 0 0 auto; min-height: 3rem; align-items: center; gap: var(--space-3); padding: var(--space-2) var(--space-5) var(--space-2) var(--space-2); border-top: 1px solid var(--border-subtle); background: transparent; }
.leading-entries, .trailing-entries { display: flex; min-width: 0; align-items: center; gap: var(--space-2); }
.leading-entries { flex: 1 1 auto; align-self: stretch; }
.trailing-entries { flex: 0 1 auto; margin-left: auto; }
.status-bar :global(.status-entry) { display: inline-flex; min-width: 0; max-width: 40vw; flex: 0 1 auto; align-items: center; gap: var(--space-2); white-space: nowrap; }
.passive { min-height: var(--control-height); padding: 0.25rem 0.375rem; border: 1px solid transparent; color: var(--text-primary); font-size: var(--font-meta); font-weight: 500; line-height: 1.2; }
.status-bar :global(button.icon-only) { width: var(--control-height); min-width: var(--control-height); height: var(--control-height); min-height: var(--control-height); flex: 0 0 var(--control-height); align-self: flex-end; justify-content: center; border: 0; border-radius: 0.625rem; background: transparent; color: var(--text-tertiary); padding: 0; }
/* Match the collection icon center: outer inset + row padding + half the icon width. */
.leading-entries :global(button.icon-only:first-child) { margin-inline-start: calc(var(--collection-icon-inset) - var(--space-2) + (var(--icon-size) - var(--control-height)) / 2); }
.status-bar :global(.app-mark-motion) { display: inline-flex; color: var(--text-secondary); opacity: 0.48; transform: scale(1); transform-origin: center; }
.status-bar :global(button.icon-only.selected .app-mark-motion) { opacity: 0.82; }
.status-bar :global(button.icon-only:focus-visible .app-mark-motion) { opacity: 0.9; }
.status-bar :global(button.icon-only:active:not(:disabled) .app-mark-motion) { opacity: 0.95; }
@media (hover: hover) and (pointer: fine) {
  .status-bar :global(button.icon-only[data-hovered]:not(:disabled, :focus-visible) .app-mark-motion) { opacity: 0.9; }
}
.status-bar :global(button.icon-only:focus-visible) { background: transparent; box-shadow: none; color: var(--text-secondary); }
.status-bar :global(button.pending:disabled) { opacity: 1; }
.entry-label-stack { display: grid; min-width: 0; max-width: 40vw; }
.entry-label, .entry-label-reserve { grid-area: 1 / 1; min-width: 0; white-space: nowrap; }
.entry-label { overflow: hidden; text-overflow: ellipsis; }
.entry-label-reserve { visibility: hidden; }
:global(.pending) .entry-label { color: var(--text-secondary); }
@media (prefers-reduced-motion: no-preference) and (hover: hover) and (pointer: fine) {
  :global(:root[data-ui-active="true"]) .status-bar :global(button.icon-only[data-hovered]:not(:disabled, :focus-visible) .app-mark-motion) { --motion-ease: var(--motion-popup-ease); transform: scale(1.16); }
}
@media (prefers-reduced-motion: no-preference) {
  :global(:root[data-ui-active="true"]) :global(.pending) .entry-label { background: linear-gradient(100deg, var(--text-secondary) 20%, var(--text-primary) 50%, var(--text-secondary) 80%); background-position: 100% 0; background-size: 250% 100%; background-clip: text; color: transparent; -webkit-background-clip: text; -webkit-text-fill-color: transparent; }
}
.separator { width: 1px; height: 1rem; margin: 0 var(--space-1); background: var(--border-subtle); }
.status-announcement { position: absolute; width: 1px; height: 1px; overflow: hidden; clip-path: inset(50%); white-space: nowrap; }
</style>
