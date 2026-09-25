<script lang="ts">
import Button from "./Button.svelte";
import Input from "./Input.svelte";
import { onMount } from "svelte";
import type { ExtensionViewSnapshot, ViewEvent } from "../types";
import CachedFileIcon from "./CachedFileIcon.svelte";
import StatusBar from "./StatusBar.svelte";
import SemanticContentIcon from "./SemanticContentIcon.svelte";
import ViewDetail from "./ViewDetail.svelte";

const { snapshot, resourceOrigin, busy, error, onQuery, onEvent, onResume, onBack, onContextMenu }: {
    snapshot: ExtensionViewSnapshot;
    resourceOrigin: string;
    busy: boolean;
    error: string | null;
    onQuery: (text: string) => void;
    onEvent: (event: ViewEvent) => Promise<number | null>;
    onResume: () => void;
    onBack: () => void;
    onContextMenu: (itemId: string | null, position: [number, number] | null) => Promise<void>;
} = $props();
const list = $derived(snapshot.view.kind === "list" ? snapshot.view.list : null);
const items = $derived(list?.sections.flatMap(section => section.items) ?? []);
let selection = $state<{ id: string; revision: number | null; token: number; } | null>(null);
let selectionToken = 0;
const selected = $derived(items.find(item => item.id === (selection?.id ?? list?.selected_item_id)) ?? null);
const detail = $derived.by(() =>
{
    if (snapshot.view.kind === "detail")
    {
        return snapshot.view.detail;
    }
    // Keep the committed preview until the Channel resolves the new selection.
    return list?.detail ?? null;
});
const detailPending = $derived(list !== null && (selected?.id ?? null) !== list.selected_item_id);
const actions = $derived(list ? selected?.actions ?? [] : detail?.actions ?? []);
const primaryAction = $derived(
    actions.find(action =>
        action.style === "primary" && action.enabled && action.allow_default_execution && !action.confirmation_title
    ) ?? null
);
const secondaryActions = $derived(actions.filter(action => action.style !== "primary"));
let confirmation = $state<{ actionId: string; itemId: string | null; revision: number; } | null>(null);
const leadingStatusEntries = $derived(secondaryActions.map(action => ({
    id: action.id,
    title: action.title,
    confirmation: action.confirmation_title
        ? { title: action.confirmation_title, active: isConfirming(action.id) }
        : undefined,
    destructive: action.style === "destructive",
    disabled: busy || !action.enabled
})));
const trailingStatusEntries = $derived(
    primaryAction
        ? [{
            id: primaryAction.id,
            title: primaryAction.title,
            interactive: false,
            keys: ["↵"],
            ariaShortcut: "Enter"
        }]
        : []
);
let query = $state("");
let input = $state<HTMLInputElement>();
let options = $state<HTMLUListElement>();
let listPane = $state<HTMLDivElement>();
let detailPane = $state<HTMLDivElement>();
let previousDetailItem: string | null | undefined;
let loadMoreSentinel = $state<HTMLDivElement>();
let requestedCursor = $state<string | null>(null);
let acknowledgedCursor = $state<string | null>(null);
let failedCursor = $state<string | null>(null);
let paginationScope = $state("");
let surface: HTMLElement;

$effect(() =>
{
    const current = confirmation;
    if (
        current && (current.revision !== snapshot.revision || current.itemId !== (selected?.id ?? null)
            || !actions.some(action => action.id === current.actionId && Boolean(action.confirmation_title)))
    )
    {
        confirmation = null;
    }
});

$effect(() =>
{
    // Reconcile only the completed request revision; earlier responses may describe an old selection.
    if (
        selection && ((selection.revision !== null && snapshot.revision >= selection.revision)
            || !items.some(item => item.id === selection?.id))
    )
    {
        selection = null;
    }
});

function selectItem(id: string): void
{
    cancelConfirmation();
    const token = ++selectionToken;
    selection = { id, revision: null, token };
    void onEvent({ kind: "selectionChanged", item_id: id }).then(revision =>
    {
        if (selection?.token === token)
        {
            selection = revision === null ? null : { id, revision, token };
        }
    });
}

$effect(() =>
{
    const item = list?.selected_item_id ?? null;
    if (detailPane && item !== previousDetailItem)
    {
        // Reset scroll only for a new record, preserving it through late thumbnail updates.
        detailPane.scrollTop = 0;
        previousDetailItem = item;
    }
});

$effect(() =>
{
    const scope = `${list?.search_text ?? ""}\u0000${list?.filter?.selected_value ?? ""}`;
    if (scope !== paginationScope)
    {
        paginationScope = scope;
        requestedCursor = null;
        acknowledgedCursor = null;
        failedCursor = null;
    }
});

$effect(() =>
{
    if (error && requestedCursor !== null && list?.next_cursor === requestedCursor)
    {
        failedCursor = requestedCursor;
        requestedCursor = null;
    }
});

$effect(() =>
{
    const cursor = list?.next_cursor ?? null;
    if (requestedCursor !== null && cursor !== requestedCursor)
    {
        acknowledgedCursor = requestedCursor;
        requestedCursor = null;
    }
});

$effect(() =>
{
    const cursor = list?.next_cursor ?? null;
    const root = listPane;
    const target = loadMoreSentinel;
    if (!cursor || !root || !target || busy || requestedCursor === cursor || acknowledgedCursor === cursor)
    {
        return;
    }
    const observer = new IntersectionObserver(entries =>
    {
        const intersecting = entries.some(entry => entry.isIntersecting);
        if (!intersecting && failedCursor === cursor)
        {
            failedCursor = null;
            return;
        }
        if (
            intersecting && !busy && requestedCursor !== cursor
            && acknowledgedCursor !== cursor && failedCursor !== cursor
        )
        {
            requestedCursor = cursor;
            onEvent({ kind: "loadMore", cursor });
        }
    }, {
        root,
        rootMargin: "0px 0px 160px 0px"
    });
    observer.observe(target);
    return () => observer.disconnect();
});

onMount(() =>
{
    query = list?.search_text ?? "";
    focusSearch();
    onResume();
});

function handleFocus(): void
{
    focusSearch();
    onResume();
}

function cancelConfirmation(): void
{
    confirmation = null;
}

function cancelConfirmationOutsideAction(event: PointerEvent): void
{
    const current = confirmation;
    if (!current || !(event.target instanceof Element))
    {
        return;
    }
    const action = event.target.closest<HTMLElement>("[data-entry-id]");
    if (action?.dataset.entryId !== current.actionId)
    {
        cancelConfirmation();
    }
}

function isConfirming(actionId: string): boolean
{
    return confirmation?.actionId === actionId
        && confirmation.itemId === (selected?.id ?? null)
        && confirmation.revision === snapshot.revision;
}

function focusSearch(): void
{
    if (input)
    {
        input.focus({ preventScroll: true });
    }
    else if (document.activeElement instanceof HTMLElement)
    {
        document.activeElement.blur();
    }
}

function activateItem(item: typeof items[number]): void
{
    if (busy)
    {
        return;
    }
    const primary = item.actions.find(action =>
        action.style === "primary" && action.enabled && action.allow_default_execution && !action.confirmation_title
    );
    if (primary)
    {
        cancelConfirmation();
        onEvent({ kind: "actionInvoked", item_id: item.id, action_id: primary.id, invocation: "default" });
    }
}

function invokeAction(actionId: string): void
{
    if (busy)
    {
        return;
    }
    const action = actions.find(candidate => candidate.id === actionId);
    if (!action || !action.enabled)
    {
        return;
    }
    focusSearch();
    const itemId = selected?.id ?? null;
    if (
        action.confirmation_title
        && !isConfirming(action.id)
    )
    {
        confirmation = { actionId: action.id, itemId, revision: snapshot.revision };
        return;
    }
    const invocation = isConfirming(action.id) ? "confirmed" : "explicit";
    cancelConfirmation();
    onEvent({ kind: "actionInvoked", item_id: itemId, action_id: action.id, invocation });
}

function handleKeydown(event: KeyboardEvent): void
{
    if (event.defaultPrevented || event.isComposing)
    {
        return;
    }
    if (event.target instanceof HTMLSelectElement)
    {
        return;
    }
    if (event.key === "Escape")
    {
        event.preventDefault();
        if (confirmation)
        {
            cancelConfirmation();
            return;
        }
        if (!busy)
        {
            onBack();
        }
        return;
    }
    const verticalNavigation = (event.key === "ArrowDown" || event.key === "ArrowUp")
        && !event.shiftKey && !event.ctrlKey && !event.altKey && !event.metaKey;
    const fromPreview = list !== null && event.target instanceof HTMLTextAreaElement
        && event.target.readOnly && surface.contains(event.target);
    const fromDetail = list === null && event.target === document.body;
    if (event.target !== input && !fromDetail && !(fromPreview && verticalNavigation))
    {
        return;
    }
    if (verticalNavigation)
    {
        event.preventDefault();
        if (busy || !list || !items.length)
        {
            return;
        }
        focusSearch();
        const index = items.findIndex(item => item.id === selected?.id);
        const next = Math.max(0, Math.min(items.length - 1, index + (event.key === "ArrowDown" ? 1 : -1)));
        const item = items[next];
        if (item)
        {
            if (item.id === selected?.id)
            {
                return;
            }
            options?.querySelectorAll<HTMLElement>('[role="option"]').item(next)?.scrollIntoView({
                block: "nearest",
                inline: "nearest",
                behavior: "instant"
            });
            selectItem(item.id);
        }
    }
    if (event.key === "Enter" && !busy)
    {
        const action = primaryAction;
        if (action)
        {
            event.preventDefault();
            onEvent({
                kind: "actionInvoked",
                item_id: selected?.id ?? null,
                action_id: action.id,
                invocation: "default"
            });
        }
    }
}

function _openContextMenu(itemId: string | null, event: MouseEvent): void
{
    event.preventDefault();
    if (busy || (list && !itemId))
    {
        return;
    }
    cancelConfirmation();
    void onContextMenu(itemId, [event.clientX, event.clientY]);
}
</script>

<svelte:window
    onpointerdown={cancelConfirmationOutsideAction}
    onkeydown={handleKeydown}
    onfocus={handleFocus}
    onblur={cancelConfirmation}
/>

<main
    class="extension-view"
    bind:this={surface}
    aria-label={list?.title ?? detail?.title ?? "Extension view"}
    oncontextmenu={list ? undefined : event => _openContextMenu(null, event)}
>
    <header class:has-filter={Boolean(list?.filter)}>
        <Button class="back" onclick={onBack} disabled={busy} aria-label="Back to previous view">
            <svg
                viewBox="0 0 24 24"
                aria-hidden="true"
                fill="none"
                stroke="currentColor"
                stroke-width="2"
                stroke-linecap="round"
                stroke-linejoin="round"
            >
                <path d="m14.5 5-7 7 7 7" />
            </svg>
        </Button>
        {#if list}
            <Input
                variant="search"
                bind:ref={input}
                bind:value={query}
                role="combobox"
                aria-label={list.search_placeholder || "Search this view"}
                placeholder={list.search_placeholder}
                aria-autocomplete="list"
                aria-controls="extension-items"
                aria-expanded={items.length > 0}
                aria-activedescendant={selected ? `view-item-${snapshot.routeId}-${selected.id}` : undefined}
                autocomplete="off"
                spellcheck="false"
                oninput={(event =>
                {
                    cancelConfirmation();
                    onQuery(event.currentTarget.value);
                })}
            />
            {#if list.filter}
                <div class="filter" role="group" aria-label="Content filter">
                    {#each list.filter.options as option (option.value)}
                        <Button
                            aria-pressed={option.value === list.filter.selected_value}
                            aria-disabled={busy}
                            onmousedown={(event => event.preventDefault())}
                            onclick={() =>
                            {
                                cancelConfirmation();
                                if (!busy && list?.filter && option.value !== list.filter.selected_value)
                                {
                                    onEvent({ kind: "filterChanged", filter_id: list.filter.id, value: option.value });
                                }
                            }}
                        >
                            {option.title}
                        </Button>
                    {/each}
                </div>
            {/if}
        {:else}
            <h1>{detail?.title ?? "Details"}</h1>
        {/if}
    </header>
    {#if error}<div class="error" role="alert">{error}</div>{/if}
    <div class="content" class:split={list?.layout === "split"} aria-busy={busy}>
        {#if list}
            <div class="list-pane" bind:this={listPane}>
                <ul bind:this={options} id="extension-items" role="listbox" aria-label={list.title}>
                    {#each list.sections as section (section.id)}
                        <li role="presentation">
                            {#if section.title}<h2>{section.title}</h2>{/if}
                            <ul role="group" aria-label={section.title ?? "Items"}>
                                {#each section.items as item (item.id)}
                                    <li
                                        id={`view-item-${snapshot.routeId}-${item.id}`}
                                        role="option"
                                        aria-selected={item.id === selected?.id}
                                        aria-disabled={busy}
                                        oncontextmenu={event =>
                                        {
                                            void _openContextMenu(item.id, event);
                                        }}
                                        onmousedown={(event => event.preventDefault())}
                                        onclick={() =>
                                        {
                                            cancelConfirmation();
                                            focusSearch();
                                            if (!busy && item.id !== selected?.id)
                                            {
                                                selectItem(item.id);
                                            }
                                        }}
                                        ondblclick={() => activateItem(item)}
                                        onkeydown={(event =>
                                        {
                                            if (event.key === "Enter" || event.key === " ")
                                            {
                                                event.preventDefault();
                                                activateItem(item);
                                            }
                                        })}
                                    >
                                        <div class="collection-row option-surface">
                                            {#if item.icon}<span class="item-icon" aria-hidden="true">
                                                    {#if typeof item.icon === "string"}
                                                        <SemanticContentIcon kind={item.icon} />
                                                    {:else}
                                                        <CachedFileIcon
                                                            reference={item.icon.native}
                                                            {resourceOrigin}
                                                            extensionId={snapshot.extensionId}
                                                        />
                                                    {/if}
                                                </span>{/if}
                                            {#if item.subtitle}
                                                <span class="item-copy"><span>{item.title}</span><small>{
                                                        item.subtitle
                                                    }</small></span>
                                            {:else}
                                                <span class="item-title">{item.title}</span>
                                            {/if}
                                        </div>
                                    </li>
                                {/each}
                            </ul>
                        </li>
                    {/each}
                </ul>
                {#if !items.length}<p class="empty">No items</p>{/if}
                {#if list.next_cursor}<div
                        class="load-more-sentinel"
                        bind:this={loadMoreSentinel}
                        aria-hidden="true"
                    >
                    </div>{/if}
            </div>
        {/if}
        {#if !list || list.layout === "split"}<div
                class="detail-pane"
                bind:this={detailPane}
                aria-busy={detailPending}
            >
                {#if detail}
                    <ViewDetail
                        {detail}
                        resourceOrigin={resourceOrigin}
                        extensionId={snapshot.extensionId}
                    />
                {/if}
            </div>{/if}
    </div>
    <StatusBar
        leadingEntries={leadingStatusEntries}
        trailingEntries={trailingStatusEntries}
        onInvoke={invokeAction}
    />
</main>

<style>
.extension-view { display: flex; flex-direction: column; width: 100%; height: 100%; overflow: hidden; border: 0; border-radius: var(--radius-window); background: var(--surface-window); color: var(--text-primary); }
/* Match Root Search's header geometry; the wider back hit target must not shift the search input. */
header { display: grid; grid-template-columns: 1rem minmax(0, 1fr); align-items: center; gap: var(--space-3); flex: 0 0 var(--search-height); height: var(--search-height); padding: 0 var(--space-5); border-bottom: 1px solid var(--border-subtle); }
header.has-filter { grid-template-columns: 1rem minmax(0, 1fr) auto; }
h1 { margin: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-size: var(--font-search); line-height: 1.2; }
/* Pending view requests keep toolbar contrast stable; native disabled and aria-busy still expose the interaction state. */
header :global(button:disabled) { opacity: 1; }
header :global(.back) { display: grid; justify-self: center; width: 2rem; height: 2rem; place-items: center; padding: 0; background: transparent; border: 0; }
header :global(.back svg) { width: 1rem; height: 1rem; }
.content { display: flex; flex: 1; min-height: 0; }
.list-pane, .detail-pane { min-width: 0; flex: 1; overflow: auto; }
.split .list-pane { flex: 0 1 38%; }
.split .detail-pane { flex: 1 1 62%; border-left: 1px solid var(--border-subtle); }
ul { list-style: none; margin: 0; padding: 0; }
.list-pane [role='group'] { display: grid; grid-template-columns: minmax(0, 1fr); }
.list-pane { padding: var(--space-2); overflow-x: hidden; }
h2 { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-size: var(--font-meta); font-weight: 500; color: var(--text-secondary); padding: var(--space-2); margin: 0; }
[role='option'] { min-width: 0; padding-top: var(--space-1); }
[role='option']:first-child { padding-top: 0; }
.option-surface { display: flex; min-width: 0; width: 100%; }
[role='option']:hover:not([aria-disabled='true']) > .option-surface { background: var(--surface-hovered); }
[role='option'][aria-selected='true'] > .option-surface, [role='option'][aria-selected='true']:hover > .option-surface { background: var(--surface-selected); }
.item-icon { display: grid; flex: 0 0 var(--icon-size); width: var(--icon-size); height: var(--icon-size); place-items: center; }
.item-copy { display: flex; min-width: 0; flex: 1; flex-direction: column; justify-content: center; line-height: 1.25; }
.item-title { min-width: 0; flex: 1; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
.item-copy > span, .item-copy > small { display: block; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
small { color: var(--text-secondary); font-size: var(--font-meta); margin-top: var(--space-1); }
.empty { color: var(--text-secondary); text-align: center; padding: var(--space-5); }
.load-more-sentinel { height: 1px; pointer-events: none; }
.filter { display: flex; align-items: center; gap: calc(var(--space-1) / 2); white-space: nowrap; }
.filter :global(button) { border-color: transparent; border-radius: 999px; background: transparent; padding: 0.35rem var(--space-2); }
.filter :global(button:hover:not(:disabled, [aria-disabled="true"])) { border-color: transparent; background: var(--surface-hovered); }
.filter :global(button[aria-pressed='true']) { background: var(--surface-selected); }
.filter :global(button[aria-pressed='true']:hover) { background: var(--surface-selected); }
.error { padding: var(--space-2) var(--space-5); font-size: var(--font-meta); }
</style>
