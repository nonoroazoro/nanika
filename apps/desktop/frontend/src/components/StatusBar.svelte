<script lang="ts">
interface StatusBarEntry
{
    id: string;
    title: string;
    interactive?: boolean;
    disabled?: boolean;
    destructive?: boolean;
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
const pendingAnnouncement = $derived.by(() =>
{
    const titles: string[] = [];
    for (const entry of leadingEntries)
    {
        if (entry.pending?.active)
        {
            titles.push(entry.pending.title);
        }
    }
    for (const entry of trailingEntries)
    {
        if (entry.pending?.active)
        {
            titles.push(entry.pending.title);
        }
    }
    return titles.join(". ");
});

function updateActivity(): void
{
    active = document.visibilityState === "visible" && document.hasFocus();
}

function handleInvoke(event: MouseEvent): void
{
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
        <span class="entry-label" aria-hidden="true">
            <span class="entry-label-value" class:hidden={Boolean(entry.pending?.active)}>{entry.title}</span>
            {#if entry.pending}<span
                    class="entry-label-value pending-value"
                    class:hidden={!entry.pending.active}
                >{entry.pending.title}</span>{/if}
        </span>
        {#each entry.keys ?? [] as key (`${entry.id}:${key}`)}
            <kbd>{key}</kbd>
        {/each}
    {/snippet}
    {#if entry.interactive === false}
        <span
            class="status-entry passive"
            class:pending={Boolean(entry.pending?.active)}
            aria-busy={entry.pending?.active ? "true" : undefined}
            aria-keyshortcuts={entry.ariaShortcut}
            aria-label={entry.pending?.active ? entry.pending.title : entry.title}
        >
            {@render content()}
        </span>
    {:else}
        <button
            class="status-entry"
            type="button"
            class:destructive={entry.destructive}
            class:pending={Boolean(entry.pending?.active)}
            data-entry-id={entry.id}
            disabled={entry.disabled || Boolean(entry.pending?.active)}
            aria-busy={entry.pending?.active ? "true" : undefined}
            aria-keyshortcuts={entry.ariaShortcut}
            aria-label={entry.pending?.active ? entry.pending.title : entry.title}
            onclick={handleInvoke}
        >
            {@render content()}
        </button>
    {/if}
{/snippet}

{#if leadingEntries.length || trailingEntries.length}
    <span class="status-announcement" role="status" aria-live="polite" aria-atomic="true">
        {pendingAnnouncement}
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
button.pending:disabled { opacity: 1; }
.entry-label { display: grid; min-width: 0; overflow: hidden; }
.entry-label-value { grid-area: 1 / 1; justify-self: end; overflow: hidden; text-overflow: ellipsis; opacity: 1; transition: opacity 140ms ease; }
.entry-label-value.hidden { opacity: 0; }
.pending .pending-value { background: linear-gradient(100deg, var(--text-secondary) 16%, color-mix(in srgb, var(--text-primary) 70%, var(--text-secondary)) 34%, light-dark(rgb(255 255 255 / 78%), rgb(255 255 255 / 94%)) 47%, light-dark(rgb(255 255 255 / 78%), rgb(255 255 255 / 94%)) 54%, color-mix(in srgb, var(--text-primary) 70%, var(--text-secondary)) 68%, var(--text-secondary) 84%); background-position: 100% 0; background-size: 250% 100%; background-clip: text; color: transparent; -webkit-background-clip: text; -webkit-text-fill-color: transparent; animation: status-bar-shimmer 1600ms ease-in-out infinite paused; }
.active .pending .pending-value { animation-play-state: running; }
.destructive { border-color: var(--border-danger); color: var(--text-danger); }
.destructive:hover:not(:disabled) { border-color: var(--border-danger-hover); background: var(--surface-danger-hover); }
.separator { width: 1px; height: 1rem; margin: 0 var(--space-1); background: var(--border-subtle); }
kbd { display: inline-grid; min-width: 1.5rem; height: 1.5rem; flex: 0 0 auto; place-items: center; padding: 0 var(--space-1); border-radius: 0.45rem; background: var(--surface-raised); color: var(--text-secondary); font: inherit; font-weight: 600; line-height: 1; }
.status-announcement { position: absolute; width: 1px; height: 1px; overflow: hidden; clip-path: inset(50%); white-space: nowrap; }

@keyframes status-bar-shimmer {
  0%, 18% { background-position: 100% 0; }
  82%, 100% { background-position: 0 0; }
}

@media (prefers-reduced-motion: reduce) {
  .entry-label-value { transition: none; }
  .pending .pending-value { background: none; color: var(--text-secondary); -webkit-text-fill-color: currentColor; animation: none; }
}
</style>
