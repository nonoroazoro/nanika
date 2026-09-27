<script lang="ts">
import SearchIcon from "./icons/SearchIcon.svelte";
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
    onInvoke: (result: SearchResult, confirmed: boolean) => void;
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
    const cancel = (): void => searchState.cancelConfirmation();
    window.addEventListener("blur", cancel);
    window.addEventListener("pointerdown", _cancelOutsideConfirmation, true);
    window.addEventListener("keydown", _cancelOnOtherKey, true);
    document.addEventListener("visibilitychange", cancel);
    document.addEventListener("compositionstart", cancel);
    return () =>
    {
        window.removeEventListener("blur", cancel);
        window.removeEventListener("pointerdown", _cancelOutsideConfirmation, true);
        window.removeEventListener("keydown", _cancelOnOtherKey, true);
        document.removeEventListener("visibilitychange", cancel);
        document.removeEventListener("compositionstart", cancel);
    };
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
    const invocation = searchState.activate(result);
    if (invocation === null)
    {
        return;
    }
    selectOnNextFocus = true;
    onInvoke(result, invocation === "confirmed");
}

function handleKeydown(event: KeyboardEvent): void
{
    if (
        event.isComposing
        || (["Enter", "ArrowUp", "ArrowDown"].includes(event.key)
            && (event.ctrlKey || event.altKey || event.metaKey || event.shiftKey))
    )
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
        if (!event.repeat)
        {
            invoke(activeResult);
        }
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
        if (searchState.confirmationTitle)
        {
            searchState.cancelConfirmation();
            return;
        }
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
        if (searchState.confirmationTitle)
        {
            searchState.cancelConfirmation();
            return;
        }
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

function _cancelOutsideConfirmation(event: PointerEvent): void
{
    if (!searchState.confirmationTitle)
    {
        return;
    }
    const row = event.target instanceof Element ? event.target.closest('[role="option"]') : null;
    // Keep the second click on the reviewed row; every other pointer press abandons it.
    if (event.button !== 0 || row?.id !== activeId || !list?.contains(row))
    {
        searchState.cancelConfirmation();
    }
}

function _cancelOnOtherKey(event: KeyboardEvent): void
{
    if (event.key === "Escape" || ["Shift", "Control", "Alt", "Meta"].includes(event.key))
    {
        return;
    }
    if (event.key !== "Enter" || event.ctrlKey || event.altKey || event.metaKey || event.shiftKey)
    {
        searchState.cancelConfirmation();
    }
}

function _openContextMenu(result: SearchResult, event: MouseEvent): void
{
    event.preventDefault();
    if (busy)
    {
        return;
    }
    searchState.cancelConfirmation();
    searchState.select(snapshot.resultOffset + results.indexOf(result));
    void onContextMenu(result, [event.clientX, event.clientY]);
}
</script>

<svelte:window onfocus={handleWindowFocus} onkeydown={handleWindowKeydown} />

<main class="launcher" aria-label="Nanika launcher">
    <div class="search-shell">
        <span class="search-icon"><SearchIcon size={16} /></span>
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
                searchState.cancelConfirmation();
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
                    searchState.cancelConfirmation();
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
                            confirmationTitle={activeResult === result ? searchState.confirmationTitle : null}
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
        {:else if snapshot.pendingExtensions.length > 0}
            <div class="empty" role="status"><span>Searching extensions…</span></div>
        {:else if hasCompletedSearch}
            <div class="empty" role="status">
                <span>No results</span>
                <small>Try another search.</small>
            </div>
        {/if}
    </section>
    <span class="confirmation-announcement" role="status" aria-live="polite" aria-atomic="true">
        {
            searchState.confirmationTitle ? `${searchState.confirmationTitle}. Press Enter to confirm or Escape to cancel.` : ""
        }
    </span>
    <StatusBar
        trailingEntries={snapshot.pendingExtensions.length
        ? [{
            id: "pending-search",
            title: `Searching ${snapshot.pendingExtensions.length} extension${
                snapshot.pendingExtensions.length === 1 ? "" : "s"
            }…`,
            interactive: false
        }]
        : []}
        leadingEntries={[{
            id: "app-menu",
            title: "Nanika menu",
            icon: "app",
            iconOnly: true,
            menu: { controls: "context-menu", expanded: appMenuOpen }
        }]}
        onInvoke={id =>
        {
            if (id === "app-menu")
            {
                searchState.cancelConfirmation();
                onAppMenu();
            }
        }}
    />
</main>

<style>
.confirmation-announcement { position: absolute; width: 1px; height: 1px; overflow: hidden; clip-path: inset(50%); white-space: nowrap; }

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

.search-icon { color: var(--text-tertiary); }

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
