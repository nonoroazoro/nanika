<script lang="ts">
import Input from "./Input.svelte";
import { onMount, tick } from "svelte";

import type { RootSearchSnapshot, SearchResult } from "../types";
import { clampIndex } from "../logic";
import StatusBar from "./StatusBar.svelte";
import ResultRow from "./ResultRow.svelte";

interface Props
{
    snapshot: RootSearchSnapshot;
    hasCompletedSearch: boolean;
    busy?: boolean;
    refreshing?: boolean;
    appMenuOpen: boolean;
    onAppMenu: () => void;
    inputError?: string | null;
    onQuery: (query: string) => void;
    onDismiss: () => void;
    onInvoke: (result: SearchResult) => void;
    onContextMenu: (result: SearchResult, position: [number, number] | null) => Promise<void>;
}

const {
    snapshot,
    hasCompletedSearch,
    busy = false,
    refreshing = false,
    inputError = null,
    onQuery,
    onDismiss,
    onInvoke,
    onContextMenu,
    appMenuOpen,
    onAppMenu
}: Props = $props();
let query = $state("");
let requestedActiveId = $state<string | null>(null);
let input = $state<HTMLInputElement>();
let list = $state<HTMLUListElement>();
let selectOnNextFocus = false;

// Transport metadata changes during submission without changing the visible list.
const results = $derived(snapshot.results);
const warning = $derived(snapshot.warnings.join("\n"));
const activeIndex = $derived(
    results.length === 0
        ? -1
        : Math.max(0, results.findIndex(result => `${result.extensionId}:${result.entryId}` === requestedActiveId))
);
const activeResult = $derived(results[activeIndex] ?? null);
const activeId = $derived(
    activeResult ? `result-${activeResult.extensionId}-${activeResult.entryId}` : undefined
);
const statusEntries = $derived([{
    id: "refresh",
    title: "Refresh",
    interactive: false,
    keys: ["F5"],
    ariaShortcut: "F5",
    pending: { title: "Refreshing", active: refreshing }
}]);

onMount(() =>
{
    query = snapshot.query;
    void tick().then(() => focusQuery(true));
});

function focusQuery(selectAll = false): void
{
    input?.focus({ preventScroll: true });
    if (selectAll)
    {
        input?.select();
    }
}

function handleWindowFocus(): void
{
    const selectAll = selectOnNextFocus;
    selectOnNextFocus = false;
    focusQuery(selectAll);
}

function invoke(result: SearchResult): void
{
    selectOnNextFocus = true;
    onInvoke(result);
}

function handleKeydown(event: KeyboardEvent): void
{
    if (event.isComposing)
    {
        return;
    }
    if (appMenuOpen)
    {
        return;
    }
    if (event.key === "ArrowDown")
    {
        event.preventDefault();
        moveSelection(1);
        return;
    }
    if (event.key === "ArrowUp")
    {
        event.preventDefault();
        moveSelection(-1);
        return;
    }
    if (event.key === "Enter" && activeResult && !busy)
    {
        event.preventDefault();
        invoke(activeResult);
        return;
    }
    if (event.key === "Tab" && !event.ctrlKey && !event.altKey && !event.metaKey)
    {
        event.preventDefault();
        if (!event.shiftKey && activeResult?.entryType === "view" && !busy)
        {
            invoke(activeResult);
        }
        return;
    }
    if (event.key === "Escape")
    {
        event.preventDefault();
        onDismiss();
    }
}

function handleWindowKeydown(event: KeyboardEvent): void
{
    if (event.defaultPrevented || event.isComposing)
    {
        return;
    }
    if (event.key === "Escape")
    {
        event.preventDefault();
        onDismiss();
        return;
    }
}

function moveSelection(delta: number): void
{
    const next = clampIndex(activeIndex + delta, results.length);
    const result = results[next];
    requestedActiveId = result ? `${result.extensionId}:${result.entryId}` : null;
    // aria-activedescendant preserves input focus but does not scroll the option.
    // Let the browser reveal only the nearest edge, only for keyboard navigation.
    list?.children.item(next)?.scrollIntoView({
        block: "nearest",
        inline: "nearest",
        behavior: "instant"
    });
}

function _openContextMenu(result: SearchResult, event: MouseEvent): void
{
    event.preventDefault();
    if (busy)
    {
        return;
    }
    requestedActiveId = `${result.extensionId}:${result.entryId}`;
    void onContextMenu(result, [event.clientX, event.clientY]);
}
</script>

<svelte:window onfocus={handleWindowFocus} onkeydown={handleWindowKeydown} />

<main class="launcher" aria-label="Nanika launcher" aria-keyshortcuts="F5">
    <div class="search-shell">
        <span class="search-icon" aria-hidden="true"></span>
        <Input
            variant="search"
            bind:ref={input}
            bind:value={query}
            role="combobox"
            aria-label="Search for apps and commands"
            aria-autocomplete="list"
            aria-invalid={inputError !== null}
            aria-describedby={inputError ? "query-error" : undefined}
            aria-controls="root-results"
            aria-expanded={results.length > 0}
            aria-activedescendant={activeId}
            autocomplete="off"
            spellcheck="false"
            placeholder="Search for apps and commands"
            oninput={(event =>
            {
                selectOnNextFocus = false;
                requestedActiveId = null;
                onQuery(event.currentTarget.value);
            })}
            onkeydown={handleKeydown}
        />
    </div>

    <section class="results" aria-label="Results" aria-busy={busy && !inputError}>
        {#if inputError}
            <div id="query-error" class="warning" role="alert">{inputError}</div>
        {/if}
        {#if warning}
            <div class="warning" role="status">
                {warning}
            </div>
        {/if}
        {#if results.length > 0}
            <ul bind:this={list} id="root-results" role="listbox">
                {#each results as result, index (`${result.extensionId}:${result.entryId}`)}
                    <ResultRow
                        {result}
                        onContextMenu={event =>
                        {
                            void _openContextMenu(result, event);
                        }}
                        active={index === activeIndex}
                        onActivate={() =>
                        {
                            requestedActiveId = `${result.extensionId}:${result.entryId}`;
                        }}
                        onInvoke={() =>
                        {
                            if (!busy)
                            {
                                invoke(result);
                            }
                        }}
                    />
                {/each}
            </ul>
        {:else if hasCompletedSearch}
            <div class="empty" role="status">
                <span>No results</span>
                <small>Try another search.</small>
            </div>
        {/if}
    </section>
    <StatusBar
        leadingEntries={[{
            id: "app-menu",
            title: "Nanika menu",
            icon: "app",
            iconOnly: true,
            menu: { controls: "context-menu", expanded: appMenuOpen }
        }]}
        trailingEntries={statusEntries}
        onInvoke={onAppMenu}
    />
</main>

<style>
.launcher {
  position: relative;
  display: grid;
  grid-template-rows: var(--search-height) minmax(0, 1fr) auto;
  width: 100%;
  height: 100%;
  overflow: hidden;
  border: 0;
  border-radius: var(--radius-window);
  background: var(--surface-window);
}

.search-shell {
  display: grid;
  grid-template-columns: 1rem minmax(0, 1fr);
  align-items: center;
  gap: var(--space-3);
  padding: 0 var(--space-5);
  border-bottom: 1px solid var(--border-subtle);
}

.search-icon {
  width: 0.75rem;
  height: 0.75rem;
  border: 1.5px solid var(--text-tertiary);
  border-radius: 50%;
  position: relative;
}

.search-icon::after {
  position: absolute;
  right: -0.28rem;
  bottom: -0.2rem;
  width: 0.36rem;
  height: 1.5px;
  border-radius: 1px;
  background: var(--text-tertiary);
  content: '';
  transform: rotate(45deg);
}

.results {
  position: relative;
  display: flex;
  flex-direction: column;
  min-height: 0;
  padding: var(--space-2);
}

ul {
  flex: 1;
  min-height: 0;
  margin: 0;
  padding: 0;
  overflow-x: hidden;
  overflow-y: auto;
  list-style: none;
  scrollbar-width: thin;
}

.empty {
  flex: 1;
  display: grid;
  height: 100%;
  place-content: center;
  gap: var(--space-1);
  color: var(--text-secondary);
  text-align: center;
}

.empty small {
  color: var(--text-tertiary);
  font-size: var(--font-meta);
}

.warning {
  padding: var(--space-2) var(--space-3);
  color: var(--text-secondary);
  font-size: var(--font-meta);
  white-space: pre-wrap;
}
</style>
