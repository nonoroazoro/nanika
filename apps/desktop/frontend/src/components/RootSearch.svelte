<script lang="ts">
import type { ReadResultsRequest } from "../types/ReadResultsRequest";
import { uiActivity } from "../ui/activity";
import ScrollArea from "./ScrollArea.svelte";
import Input from "./Input.svelte";
import { onMount, tick } from "svelte";

import { RootSearchState } from "../logic/RootSearchState.svelte";
import type { SearchResult } from "../types";
import { clampIndex } from "../logic";
import StatusBar from "./StatusBar.svelte";
import ResultRow from "./ResultRow.svelte";

interface Props
{
    searchState: RootSearchState;
    hasCompletedSearch: boolean;
    busy?: boolean;
    appMenuOpen: boolean;
    onAppMenu: () => void;
    inputError?: string | null;
    onQuery: (query: string) => void;
    onRange: (request: ReadResultsRequest) => void;
    onDismiss: () => void;
    onInvoke: (result: SearchResult) => void;
    onContextMenu: (result: SearchResult, position: [number, number] | null) => Promise<void>;
}

const {
    searchState,
    hasCompletedSearch,
    busy = false,
    inputError = null,
    onQuery,
    onRange,
    onDismiss,
    onInvoke,
    onContextMenu,
    appMenuOpen,
    onAppMenu
}: Props = $props();
let query = $state("");
let input = $state<HTMLInputElement>();
let list = $state<HTMLDivElement | null>(null);
let selectOnNextFocus = false;

let viewportHeight = $state(520);
let rowHeight = $state(52);
let lastRange = "";
const snapshot = $derived(searchState.snapshot);
const results = $derived(snapshot.results);
const warning = $derived(snapshot.warnings.join("\n"));
const activeIndex = $derived(searchState.selectedIndex);
const activeResult = $derived(searchState.selectedResult);
const activeId = $derived(activeResult ? `result-${activeIndex + 1}` : undefined);
const overscan = 6;
const start = $derived(Math.max(0, Math.floor(searchState.scrollTop / rowHeight) - overscan));
const count = $derived(Math.ceil(viewportHeight / rowHeight) + overscan * 2);

// Keep the native viewport synchronized with the displayed result window.
// Only a completed new query resets the model's scroll position.
$effect.pre(() =>
{
    if (list && list.scrollTop !== searchState.scrollTop)
    {
        list.scrollTop = searchState.scrollTop;
    }
});

// ResizeObserver owns viewport synchronization; ranking and rows remain host-owned.
$effect(() =>
{
    if (!list)
    {
        return;
    }
    const element = list;
    const observer = new ResizeObserver(() =>
    {
        if (element.clientHeight > 0)
        {
            viewportHeight = element.clientHeight;
            const measured = element.querySelector("[role=option]")?.getBoundingClientRect().height;
            if (measured && measured > 0)
            {
                rowHeight = measured;
            }
        }
    });
    observer.observe(element);
    return () => observer.disconnect();
});

$effect(() =>
{
    if (!$uiActivity.visible || busy || snapshot.totalResults === 0)
    {
        return;
    }
    const key = `${snapshot.requestId}:${snapshot.resultRevision}:${start}:${count}`;
    if (key !== lastRange)
    {
        lastRange = key;
        onRange({
            sessionId: snapshot.sessionId,
            requestId: snapshot.requestId,
            resultRevision: snapshot.resultRevision,
            offset: start,
            count
        });
    }
});

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
    const next = clampIndex(activeIndex + delta, snapshot.totalResults);
    searchState.select(next);
    if (list)
    {
        const top = next * rowHeight;
        if (top < list.scrollTop)
        {
            list.scrollTop = top;
        }
        else if (top + rowHeight > list.scrollTop + list.clientHeight)
        {
            list.scrollTop = top + rowHeight - list.clientHeight;
        }
        searchState.scrollTop = list.scrollTop;
    }
}

function _openContextMenu(result: SearchResult, event: MouseEvent): void
{
    event.preventDefault();
    if (busy)
    {
        return;
    }
    searchState.select(snapshot.resultOffset + results.indexOf(result));
    void onContextMenu(result, [event.clientX, event.clientY]);
}
</script>

<svelte:window onfocus={handleWindowFocus} onkeydown={handleWindowKeydown} />

<main class="launcher" aria-label="Nanika launcher">
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
            <ScrollArea
                bind:viewport={list}
                viewportClass="root-viewport"
                onscroll={() =>
                {
                    searchState.scrollTop = list?.scrollTop ?? 0;
                }}
            >
                <ul
                    id="root-results"
                    role="listbox"
                >
                    <li role="presentation" aria-hidden="true" style:height={`${snapshot.resultOffset * rowHeight}px`}>
                    </li>
                    {#each results as result, index (RootSearchState.identity(result))}
                        <ResultRow
                            {result}
                            position={snapshot.resultOffset + index + 1}
                            total={snapshot.totalResults}
                            onContextMenu={event =>
                            {
                                void _openContextMenu(result, event);
                            }}
                            active={activeResult !== null && snapshot.resultOffset + index === activeIndex}
                            onActivate={() =>
                            {
                                searchState.select(snapshot.resultOffset + index);
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
                    <li
                        role="presentation"
                        aria-hidden="true"
                        style:height={`${Math.max(0, snapshot.totalResults - snapshot.resultOffset - results.length) * rowHeight}px`}
                    >
                    </li>
                </ul>
            </ScrollArea>
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

:global(.root-viewport) { overflow-anchor: none; }

ul {
  flex: 1;
  min-height: 0;
  margin: 0;
  padding: 0;
  list-style: none;
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
