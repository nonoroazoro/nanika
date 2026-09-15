<script lang="ts">
import { onMount } from "svelte";

import type { RootSearchSnapshot, SearchResult } from "../types";
import { clampIndex } from "../logic";
import ResultRow from "./ResultRow.svelte";

interface Props
{
    snapshot: RootSearchSnapshot;
    hasCompletedSearch: boolean;
    busy?: boolean;
    refreshing?: boolean;
    onRefresh: () => void;
    inputError?: string | null;
    onQuery: (query: string) => void;
    onDismiss: () => void;
    onInvoke: (result: SearchResult) => void;
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
    onRefresh
}: Props = $props();
let query = $state("");
let requestedActiveIndex = $state(0);
let input: HTMLInputElement;
let list = $state<HTMLUListElement>();

// Transport metadata changes during submission without changing the visible list.
const results = $derived(snapshot.results);
const warning = $derived(snapshot.warnings.join("\n"));
const activeIndex = $derived(
    results.length === 0 ? -1 : clampIndex(requestedActiveIndex, results.length)
);
const activeResult = $derived(results[activeIndex] ?? null);
const activeId = $derived(
    activeResult ? `result-${activeResult.extensionId}-${activeResult.entryId}` : undefined
);

onMount(() =>
{
    query = snapshot.query;
    focusQuery();
});

function focusQuery(): void
{
    input.focus({ preventScroll: true });
    input.select();
}

function handleKeydown(event: KeyboardEvent): void
{
    if (event.isComposing)
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
        onInvoke(activeResult);
        return;
    }
    if (event.key === "Escape")
    {
        event.preventDefault();
        onDismiss();
    }
}

function handleRefreshKey(event: KeyboardEvent): void
{
    if (
        event.key !== "F5" || event.ctrlKey || event.altKey || event.metaKey || event.shiftKey
        || event.isComposing || document.visibilityState !== "visible" || !document.hasFocus()
    )
    {
        return;
    }
    // This handler exists only while Root Search is mounted. Suppress WebView
    // reload even for key repeat or an operation already in progress.
    event.preventDefault();
    if (!event.repeat && !refreshing)
    {
        onRefresh();
    }
}

function moveSelection(delta: number): void
{
    requestedActiveIndex = clampIndex(activeIndex + delta, results.length);
    // aria-activedescendant preserves input focus but does not scroll the option.
    // Let the browser reveal only the nearest edge, only for keyboard navigation.
    list?.children.item(requestedActiveIndex)?.scrollIntoView({
        block: "nearest",
        inline: "nearest",
        behavior: "instant"
    });
}
</script>

<svelte:window onfocus={focusQuery} onkeydown={handleRefreshKey} />

<main class="launcher" aria-label="Nanika launcher" aria-keyshortcuts="F5">
    <div class="search-shell">
        <span class="search-icon" aria-hidden="true"></span>
        <input
            bind:this={input}
            bind:value={query}
            role="combobox"
            aria-label="Search apps and commands"
            aria-autocomplete="list"
            aria-invalid={inputError !== null}
            aria-describedby={inputError ? "query-error" : undefined}
            aria-controls="root-results"
            aria-expanded={results.length > 0}
            aria-activedescendant={activeId}
            autocomplete="off"
            spellcheck="false"
            placeholder="Search apps and commands"
            oninput={(event =>
            {
                requestedActiveIndex = 0;
                onQuery(event.currentTarget.value);
            })}
            onkeydown={handleKeydown}
        />
    </div>

    <section class="results" aria-label="Results" aria-busy={busy && !inputError}>
        {#if refreshing}
            <span class="refresh-status" role="status">Refreshing…</span>
        {/if}
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
                        active={index === activeIndex}
                        onActivate={() =>
                        {
                            requestedActiveIndex = index;
                        }}
                        onInvoke={() =>
                        {
                            if (!busy)
                            {
                                onInvoke(result);
                            }
                        }}
                    />
                {/each}
            </ul>
        {:else if hasCompletedSearch}
            <div class="empty" role="status">
                <span>No results</span>
                <small>Enable an extension or try another search.</small>
            </div>
        {/if}
    </section>
</main>

<style>
.launcher {
  display: grid;
  grid-template-rows: var(--search-height) minmax(0, 1fr);
  width: 100%;
  height: 100%;
  overflow: hidden;
  border: 1px solid var(--border-window);
  border-radius: var(--radius-window);
  background: var(--surface-window);
  box-shadow: var(--shadow-window);
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

input {
  width: 100%;
  height: 100%;
  border: 0;
  outline: 0;
  background: transparent;
  color: var(--text-primary);
  font: inherit;
  font-size: var(--font-search);
  caret-color: var(--accent);
}

input::placeholder {
  color: var(--text-tertiary);
  opacity: 1;
}

.results {
  position: relative;
  display: flex;
  flex-direction: column;
  min-height: 0;
  padding: var(--space-2);
}

.refresh-status {
  position: absolute;
  right: var(--space-3);
  bottom: var(--space-2);
  z-index: 1;
  padding: var(--space-1) var(--space-2);
  border-radius: var(--radius-row);
  background: var(--surface-raised);
  color: var(--text-secondary);
  font-size: var(--font-meta);
  pointer-events: none;
}

ul {
  flex: 1;
  min-height: 0;
  margin: 0;
  padding: 0;
  overflow: auto;
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
