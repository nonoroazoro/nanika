<script lang="ts">
import { onMount } from "svelte";

import type { RootSearchSnapshot, SearchResult } from "../types";
import { clampIndex } from "../logic";
import ResultRow from "./ResultRow.svelte";

interface Props
{
    snapshot: RootSearchSnapshot;
    busy?: boolean;
    inputError?: string | null;
    onQuery: (query: string) => void;
    onDismiss: () => void;
    onInvoke: (result: SearchResult) => void;
}

const { snapshot, busy = false, inputError = null, onQuery, onDismiss, onInvoke }: Props = $props();
let query = $state("");
let requestedActiveIndex = $state(0);
let input: HTMLInputElement;
let list = $state<HTMLUListElement>();

const activeIndex = $derived(
    snapshot.results.length === 0 ? -1 : clampIndex(requestedActiveIndex, snapshot.results.length)
);
const activeResult = $derived(snapshot.results[activeIndex] ?? null);
const activeId = $derived(
    activeResult ? `result-${activeResult.extensionId}-${activeResult.entryId}` : undefined
);

onMount(() =>
{
    query = snapshot.query;
    input.focus();
    if (query.length > 0)
    {
        input.select();
    }
});

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

function moveSelection(delta: number): void
{
    requestedActiveIndex = clampIndex(activeIndex + delta, snapshot.results.length);
    // aria-activedescendant preserves input focus but does not scroll the option.
    // Let the browser reveal only the nearest edge, only for keyboard navigation.
    list?.children.item(requestedActiveIndex)?.scrollIntoView({
        block: "nearest",
        inline: "nearest",
        behavior: "instant"
    });
}
</script>

<main class="launcher" aria-label="Nanika launcher">
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
            aria-expanded={snapshot.results.length > 0}
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
        {#if inputError}
            <div id="query-error" class="warning" role="alert">{inputError}</div>
        {/if}
        {#if snapshot.warnings.length > 0}
            <div class="warning" role="status">
                {snapshot.warnings.join("\n")}
            </div>
        {/if}
        {#if snapshot.results.length > 0}
            <ul bind:this={list} id="root-results" role="listbox">
                {#each snapshot.results as result, index (`${result.extensionId}:${result.entryId}`)}
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
        {:else if snapshot.phase === "ready" && !busy}
            <div class="empty" role="status">
                <span>No results</span>
                <small>Enable an extension or try another search.</small>
            </div>
        {:else}
            <div class="empty" role="status">Searching...</div>
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
