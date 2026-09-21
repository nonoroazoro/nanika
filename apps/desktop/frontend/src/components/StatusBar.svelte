<script lang="ts">
import ShortcutKeys from "./ShortcutKeys.svelte";

interface StatusBarEntry
{
    id: string;
    title: string;
    icon?: "app";
    iconOnly?: boolean;
    menu?: {
        controls: string;
        expanded: boolean;
    };
    interactive?: boolean;
    disabled?: boolean;
    destructive?: boolean;
    confirmation?: {
        title: string;
        active: boolean;
    };
    keys?: string[];
    ariaShortcut?: string;
    pending?: {
        title: string;
        active: boolean;
    };
}

const { leadingEntries = [], trailingEntries = [], onInvoke }: {
    leadingEntries?: StatusBarEntry[];
    trailingEntries?: StatusBarEntry[];
    onInvoke?: (id: string) => void;
} = $props();
let active = $state(document.visibilityState === "visible" && document.hasFocus());
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

function updateActivity(): void
{
    active = document.visibilityState === "visible" && document.hasFocus();
}

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

<svelte:document onvisibilitychange={updateActivity} />
<svelte:window onfocus={updateActivity} onblur={updateActivity} />

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
            class:pending={Boolean(entry.pending?.active)}
            aria-busy={entry.pending?.active ? "true" : undefined}
            aria-keyshortcuts={entry.ariaShortcut}
            aria-label={displayedTitle(entry)}
        >
            {@render content()}
        </span>
    {:else}
        <button
            class="status-entry"
            class:icon-only={entry.iconOnly}
            class:selected={entry.menu?.expanded}
            type="button"
            class:destructive={entry.destructive}
            class:pending={Boolean(entry.pending?.active)}
            data-entry-id={entry.id}
            disabled={entry.disabled || Boolean(entry.pending?.active)}
            aria-busy={entry.pending?.active ? "true" : undefined}
            aria-keyshortcuts={entry.ariaShortcut}
            aria-label={displayedTitle(entry)}
            aria-haspopup={entry.menu ? "menu" : undefined}
            aria-expanded={entry.menu?.expanded}
            aria-controls={entry.menu?.controls}
            onclick={handleInvoke}
        >
            {@render content()}
        </button>
    {/if}
{/snippet}

{#if leadingEntries.length || trailingEntries.length}
    <span class="status-announcement" role="status" aria-live="polite" aria-atomic="true">
        {statusAnnouncement}
    </span>
    <!-- Preserve editing focus on pointer press; button click actions still run. -->
    <footer
        class="status-bar"
        class:active
        role="status"
        aria-live="off"
        aria-label="Status bar"
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
.status-entry { display: inline-flex; min-width: 0; max-width: 40vw; min-height: 2rem; flex: 0 1 auto; align-items: center; gap: var(--space-2); border: 1px solid transparent; border-radius: var(--radius-row); background: transparent; color: var(--text-primary); padding: 0.25rem 0.375rem; font-size: var(--font-meta); font-weight: 500; line-height: 1.2; white-space: nowrap; }
button:hover:not(:disabled) { border-color: transparent; background: var(--surface-hovered); }
button.icon-only { width: var(--icon-size); min-width: var(--icon-size); height: var(--icon-size); min-height: var(--icon-size); flex: 0 0 var(--icon-size); justify-content: center; border: 0; border-radius: 0.625rem; background: transparent; color: var(--text-tertiary); padding: 0; }
button.icon-only .app-mark { opacity: 0.48; transform-origin: center bottom; transform-box: fill-box; transition: opacity 120ms ease, color 120ms ease; }
button.icon-only:hover:not(:disabled), button.icon-only.selected { background: transparent; color: var(--text-secondary); }
button.icon-only:hover:not(:disabled) .app-mark { opacity: 0.82; animation: app-mark-hop 520ms both; }
button.icon-only.selected .app-mark { opacity: 0.82; }
button.icon-only:focus-visible { background: transparent; box-shadow: none; color: var(--text-secondary); }
button.icon-only:focus-visible .app-mark { opacity: 0.9; }
button.icon-only:active:not(:disabled) .app-mark { opacity: 0.95; transform: translateY(1px) scaleX(1.045) scaleY(0.9); transition-duration: 70ms; animation: none; }
button.pending:disabled { opacity: 1; }
.entry-label-stack { display: grid; min-width: 0; max-width: 40vw; }
.entry-label, .entry-label-reserve { grid-area: 1 / 1; min-width: 0; white-space: nowrap; }
.entry-label { overflow: hidden; text-overflow: ellipsis; }
.entry-label-reserve { visibility: hidden; }
.pending .entry-label { background: linear-gradient(100deg, var(--text-secondary) 16%, color-mix(in srgb, var(--text-primary) 70%, var(--text-secondary)) 34%, light-dark(rgb(255 255 255 / 78%), rgb(255 255 255 / 94%)) 47%, light-dark(rgb(255 255 255 / 78%), rgb(255 255 255 / 94%)) 54%, color-mix(in srgb, var(--text-primary) 70%, var(--text-secondary)) 68%, var(--text-secondary) 84%); background-position: 100% 0; background-size: 250% 100%; background-clip: text; color: transparent; -webkit-background-clip: text; -webkit-text-fill-color: transparent; animation: status-bar-shimmer 1600ms ease-in-out infinite paused; }
.active .pending .entry-label { animation-play-state: running; }
.destructive { border-color: var(--border-danger); color: var(--text-danger); }
.destructive:hover:not(:disabled) { border-color: var(--border-danger-hover); background: var(--surface-danger-hover); }
.separator { width: 1px; height: 1rem; margin: 0 var(--space-1); background: var(--border-subtle); }
.status-announcement { position: absolute; width: 1px; height: 1px; overflow: hidden; clip-path: inset(50%); white-space: nowrap; }

@keyframes status-bar-shimmer {
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

@media (prefers-reduced-motion: reduce) {
  button.icon-only:hover:not(:disabled) .app-mark { animation: none; }
  button.icon-only:active:not(:disabled) .app-mark { transform: none; }
  .pending .entry-label { background: none; color: var(--text-secondary); -webkit-text-fill-color: currentColor; animation: none; }
}
</style>
