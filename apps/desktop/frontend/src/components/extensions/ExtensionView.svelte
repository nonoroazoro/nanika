<script lang="ts">
import { operationErrorText, operationFailure } from "../../localization/operation-errors";
import type { OperationError } from "../../generated/OperationError";
import { localization } from "../../localization/state.svelte";
import type { TextPresentation } from "../../localization/TextPresentation";
import { isCompositionKey } from "../../ui/keyboard";
import { initialListItemCount } from "../../ui/list-loading";
import ChevronLeftIcon from "../icons/ChevronLeftIcon.svelte";
import SearchIcon from "../icons/SearchIcon.svelte";
import FolderOpenIcon from "../icons/FolderOpenIcon.svelte";
import EmptyState from "../ui/EmptyState.svelte";
import ScrollArea from "../ui/ScrollArea.svelte";
import Button from "../ui/Button.svelte";
import Input from "../ui/Input.svelte";
import { confirmAction } from "../../ui/confirmation";
import { onDestroy, onMount, untrack } from "svelte";
import { scrollGeometry } from "../../ui/scroll-geometry";
import { RangeReadState } from "../../ui/RangeReadState.svelte";
import { uiActivity } from "../../ui/activity";
import { collectionWindow, itemAt, itemIndex, itemTop } from "../../ui/collection-window";
import type { ListItem } from "../../generated/ListItem";
import type { DetailView } from "../../generated/DetailView";
import type { ExtensionViewSnapshot } from "../../generated/ExtensionViewSnapshot";
import type { ViewInteraction } from "../../types/ViewInteraction";
import CachedFileIcon from "./CachedFileIcon.svelte";
import StatusBar from "../ui/StatusBar.svelte";
import SemanticContentIcon from "./SemanticContentIcon.svelte";
import ViewDetail from "./ViewDetail.svelte";

import type { TextDocument } from "../../generated/TextDocument";
import type { TextRead } from "../../generated/TextRead";
import type { TextChunk } from "../../generated/TextChunk";

const {
    textScope,
    onReadText,
    snapshot,
    extensionName,
    presentation,
    query,
    resourceOrigin,
    busy,
    canLoadMore,
    error,
    onQuery,
    onEvent,
    onResume,
    onBack,
    onContextMenu,
    menuOpen,
    onMenu
}: {
    textScope: string;
    onReadText: (read: TextRead) => Promise<TextChunk>;
    snapshot: ExtensionViewSnapshot;
    extensionName: string;
    presentation: TextPresentation;
    query: string;
    resourceOrigin: string;
    busy: boolean;
    canLoadMore: boolean;
    error: string | null;
    onQuery: (text: string, minimumItems: number) => void;
    onEvent: (event: ViewInteraction) => Promise<number | null>;
    onResume: () => void;
    onBack: () => void;
    menuOpen: boolean;
    onMenu: (trigger: HTMLButtonElement) => void;
    onContextMenu: (itemId: string | null, position: [number, number] | null) => Promise<void>;
} = $props();
const list = $derived(snapshot.view.kind === "list" ? snapshot.view.list : null);
const items = $derived(list?.sections.flatMap(section => section.items) ?? []);
let selection = $state<
    { item: ListItem; presentation: TextPresentation; index: number; revision: number | null; token: number; } | null
>(null);
let selectionToken = 0;
const selected = $derived(selection?.item ?? list?.selection?.item ?? null);
const selectedIndex = $derived(selection?.index ?? list?.selection?.index ?? null);
const totalItems = $derived(list?.sections.reduce((sum, section) => sum + section.total, 0) ?? 0);
const detailPending = $derived(list !== null && (selected?.id ?? null) !== (list.selection?.item.id ?? null));
let settledPreview = $state.raw<
    { itemId: string | null; detail: DetailView | null; presentation: TextPresentation; } | null
>(null);
const preview = $derived(
    detailPending
        ? settledPreview
        : { itemId: list?.selection?.item.id ?? null, detail: list?.detail ?? null, presentation }
);
const detail = $derived(snapshot.view.kind === "detail" ? snapshot.view.detail : preview?.detail ?? null);

$effect(() =>
{
    // Channel updates for earlier selections must not replace the last settled preview.
    // Keep its identity with its content so stale responses cannot reset detail scrolling either.
    if (list && !detailPending)
    {
        settledPreview = { itemId: list.selection?.item.id ?? null, detail: list.detail, presentation };
    }
});
const detailPresentation = $derived(
    snapshot.view.kind === "detail" ? presentation : preview?.presentation ?? presentation
);
const detailTitle = $derived(detail?.title ? detailPresentation.text(detail.title, localization.locale) : null);
const actionPresentation = $derived(list && selection ? selection.presentation : presentation);
const actions = $derived(list ? selected?.actions ?? [] : detail?.actions ?? []);
const primaryAction = $derived(
    actions.find(action =>
        action.style === "primary" && action.enabled && action.allow_default_execution && !action.confirmation_title
    ) ?? null
);
const secondaryActions = $derived(actions.filter(action => action.style !== "primary"));
let alive = true;
let confirming = $state(false);
let confirmationError = $state.raw<OperationError | null>(null);
const confirmationErrorText = $derived(
    confirmationError
        ? operationErrorText(confirmationError, localization.locale, value => localization.text(value))
        : null
);
onDestroy(() =>
{
    alive = false;
});
const leadingStatusEntries = $derived(secondaryActions.map(action => ({
    id: `action:${action.id}`,
    title: actionPresentation.text(action.title, localization.locale),
    destructive: action.style === "destructive",
    disabled: busy || confirming || !action.enabled
})));
let detectedLanguage = $state<{ document: TextDocument; label: string | null; } | null>(null);
// Follow the displayed preview, which remains visible while a selection is pending.
const languageLabel = $derived(
    detail?.content.kind === "text" && detail.content.syntax_highlighting
        && detectedLanguage?.document.id === detail.content.document.id
        && detectedLanguage.document.revision === detail.content.document.revision
        ? detectedLanguage.label
        : null
);
const trailingStatusEntries = $derived(
    primaryAction
        ? [{
            id: `action:${primaryAction.id}`,
            title: actionPresentation.text(primaryAction.title, localization.locale),
            interactive: list === null,
            disabled: busy,
            keys: ["↵"],
            ariaShortcut: "Enter"
        }]
        : []
);
let input = $state<HTMLInputElement>();
let listPane = $state<HTMLDivElement | null>(null);
let detailPane = $state<HTMLDivElement | null>(null);
let previousDetailItem: string | null | undefined;
let listContainer = $state<HTMLDivElement>();
let rowMeasure = $state<HTMLDivElement>();
let headingMeasure = $state<HTMLDivElement>();
let scrollTop = $state(0);
// Native scroll positions are quantized. Preserve the exact logical destination until native input moves.
let scrollAnchor = $state<{ physical: number; logical: number; } | null>(null);
let viewportHeight = $state(0);
let rowHeight = $state(52);
let headingHeight = $state(36);
let pendingKeyboard = $state<{ collectionId: string; index: number; } | null>(null);
const rangeRead = new RangeReadState();
const sectionStarts = $derived.by(() =>
{
    let index = 0;
    let top = 0;
    return list?.sections.map(section =>
    {
        const layout = { index, top };
        index += section.total;
        top += (section.title ? headingHeight : 0) + section.total * rowHeight;
        return layout;
    }) ?? [];
});
const logicalHeight = $derived(
    totalItems * rowHeight + (list?.sections.filter(section => section.title).length ?? 0) * headingHeight
);
const geometry = $derived(scrollGeometry(logicalHeight, viewportHeight));
const logicalTop = $derived(
    scrollAnchor?.physical === scrollTop
        ? Math.min(scrollAnchor.logical, Math.max(0, logicalHeight - viewportHeight))
        : geometry.logical(scrollTop)
);
const coordinateShift = $derived(geometry.coordinateShift(scrollTop, logicalTop));

$effect(() =>
{
    const pane = listPane;
    if (!pane || logicalHeight <= geometry.extent)
    {
        return;
    }
    // Only thumb/track navigation uses the compressed range. Wheel deltas already
    // describe content displacement and must not acquire the track's scale factor.
    pane.addEventListener("wheel", _wheel, { passive: false });
    return () => pane.removeEventListener("wheel", _wheel);
});

$effect(() =>
{
    const pane = listPane;
    const measure = rowMeasure;
    const heading = headingMeasure;
    if (!pane || !measure || !heading)
    {
        return;
    }
    const observer = new ResizeObserver(() =>
    {
        viewportHeight = pane.clientHeight;
        rowHeight = measure.getBoundingClientRect().height;
        headingHeight = heading.getBoundingClientRect().height;
    });
    observer.observe(pane);
    observer.observe(measure);
    observer.observe(heading);
    return () => observer.disconnect();
});

let listScope: string | null = null;
$effect(() =>
{
    const scope = JSON.stringify([snapshot.routeId, list?.search_text, list?.filter?.selected_value]);
    if (listPane && listScope !== scope)
    {
        listPane.scrollTop = 0;
        scrollTop = 0;
        scrollAnchor = null;
        pendingKeyboard = null;
        rangeRead.reset();
        listScope = scope;
    }
});

$effect(() =>
{
    if (!list || !listPane || viewportHeight <= 0 || rowHeight <= 0 || !$uiActivity.visible || !canLoadMore)
    {
        return;
    }
    if (pendingKeyboard && pendingKeyboard.collectionId !== list.collection_id)
    {
        pendingKeyboard = null;
    }
    const keyboard = pendingKeyboard?.index ?? null;
    if (keyboard !== null)
    {
        const item = itemAt(list.sections, keyboard);
        if (item)
        {
            pendingKeyboard = null;
            selectItem(item.id);
            _revealItem(item.id);
            return;
        }
    }
    const range = collectionWindow(list.sections, logicalTop, viewportHeight, rowHeight, headingHeight);
    if (range.total === 0 || (range.covered && keyboard === null))
    {
        return;
    }
    const offset = keyboard === null ? range.offset : Math.max(0, keyboard - Math.floor(range.count / 2));
    const collectionId = list.collection_id;
    const key = JSON.stringify([collectionId, offset, range.count]);
    void rangeRead.read(key, () =>
        onEvent({
            kind: "listRangeChanged",
            collection_id: collectionId,
            offset,
            count: range.count
        }));
});

$effect(() =>
{
    // Reconcile only the completed request revision; earlier responses may describe an old selection.
    if (
        selection && selection.revision !== null && snapshot.revision >= selection.revision
    )
    {
        selection = null;
    }
});

// Preserve the first shared record after DOM geometry updates and before paint.
// Range replacements keep collection identity; query changes intentionally reset to the top.
let previousCollection: { id: string; scope: string; sections: NonNullable<typeof list>["sections"]; } | null = null;
$effect(() =>
{
    const current = list;
    if (!current)
    {
        previousCollection = null;
        return;
    }
    const scope = JSON.stringify([snapshot.routeId, current.search_text, current.filter?.selected_value]);
    untrack(() =>
    {
        if (
            previousCollection && previousCollection.id !== current.collection_id && previousCollection.scope === scope
            && listPane && listPane.scrollTop > 0
        )
        {
            const anchor = previousCollection.sections.flatMap(section => section.items).find(item =>
                itemIndex(current.sections, item.id) !== null
            );
            if (anchor)
            {
                const before = itemIndex(previousCollection.sections, anchor.id)!;
                const after = itemIndex(current.sections, anchor.id)!;
                const oldHeight = previousCollection.sections.reduce(
                    (height, section) => height + (section.total * rowHeight) + (section.title ? headingHeight : 0),
                    0
                );
                const oldTop = scrollAnchor?.physical === scrollTop
                    ? scrollAnchor.logical
                    : scrollGeometry(oldHeight, viewportHeight).logical(scrollTop);
                _scrollToLogical(
                    oldTop + itemTop(current.sections, after, rowHeight, headingHeight)
                        - itemTop(previousCollection.sections, before, rowHeight, headingHeight)
                );
            }
        }
        previousCollection = { id: current.collection_id, scope, sections: current.sections };
    });
});

function _revealItem(id: string): void
{
    if (!list || !listPane)
    {
        return;
    }
    const index = itemIndex(list.sections, id);
    if (index === null)
    {
        return;
    }
    const top = itemTop(list.sections, index, rowHeight, headingHeight);
    const currentTop = logicalTop;
    const nextTop = top < currentTop ? top : Math.max(currentTop, top + rowHeight - listPane.clientHeight);
    _scrollToLogical(nextTop);
}

function _scrollToLogical(top: number): void
{
    if (!listPane)
    {
        return;
    }
    const logical = Math.max(0, Math.min(Math.max(0, logicalHeight - viewportHeight), top));
    listPane.scrollTop = geometry.physical(logical);
    scrollTop = listPane.scrollTop;
    scrollAnchor = { physical: scrollTop, logical };
}

function _wheel(event: WheelEvent): void
{
    if (event.ctrlKey || event.shiftKey || !event.cancelable || event.deltaY === 0)
    {
        return;
    }
    let unit = 1;
    if (event.deltaMode === WheelEvent.DOM_DELTA_LINE)
    {
        unit = rowHeight;
    }
    else if (event.deltaMode === WheelEvent.DOM_DELTA_PAGE)
    {
        unit = viewportHeight;
    }
    event.preventDefault();
    _scrollToLogical(logicalTop + event.deltaY * unit);
}

function _initialItemCount(): number
{
    if (!listContainer || !rowMeasure)
    {
        throw new Error("List geometry is unavailable.");
    }
    // Measure even an empty result set; the probe uses the same token as collection rows.
    const style = getComputedStyle(listContainer);
    const height = listContainer.clientHeight - Number.parseFloat(style.paddingTop)
        - Number.parseFloat(style.paddingBottom);
    return initialListItemCount(height, rowMeasure.getBoundingClientRect().height);
}

function selectItem(id: string): void
{
    pendingKeyboard = null;
    const token = ++selectionToken;
    const item = items.find(candidate => candidate.id === id) ?? list?.selection?.item;
    const index = list ? itemIndex(list.sections, id) ?? list.selection?.index : null;
    if (!list || !item || item.id !== id || index === null || index === undefined)
    {
        return;
    }
    const selectedPresentation = presentation;
    selection = { item, index, revision: null, token, presentation: selectedPresentation };
    void onEvent({ kind: "selectionChanged", collection_id: list.collection_id, index }).then(revision =>
    {
        if (selection?.token === token)
        {
            selection = revision === null ? null : { item, index, revision, token, presentation: selectedPresentation };
        }
    });
}

$effect(() =>
{
    const item = preview?.itemId ?? null;
    if (detailPane && item !== previousDetailItem)
    {
        // Reset scroll only for a new record, preserving it through late thumbnail updates.
        detailPane.scrollTop = 0;
        previousDetailItem = item;
    }
});

onMount(() =>
{
    focusSearch();
    onResume();
});

function handleFocus(): void
{
    focusSearch();
    onResume();
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
        onEvent({ kind: "actionInvoked", item_id: item.id, action_id: primary.id, invocation: "default" });
    }
}

async function invokeAction(actionId: string): Promise<void>
{
    if (busy || confirming)
    {
        return;
    }
    const action = actions.find(candidate => candidate.id === actionId);
    if (!action || !action.enabled)
    {
        return;
    }
    const current = snapshot;
    const itemId = selected?.id ?? null;
    confirming = true;
    confirmationError = null;
    try
    {
        const prepared = actionPresentation;
        const confirmed = action.confirmation_title
            ? await confirmAction({
                content: () => ({
                    title: prepared.text(action.confirmation_title!, localization.locale),
                    label: prepared.text(action.title, localization.locale),
                    message: action.confirmation_description
                        ? prepared.text(action.confirmation_description, localization.locale)
                        : ""
                }),
                destructive: action.style === "destructive",
                icon: action.icon
                    ? { kind: "symbol", name: action.icon }
                    : { kind: "image", url: snapshot.extensionIconUrl }
            })
            : false;
        if (
            !alive || busy || snapshot.routeId !== current.routeId || snapshot.instanceId !== current.instanceId
            || snapshot.revision !== current.revision || itemId !== (selected?.id ?? null)
            || (action.confirmation_title && !confirmed)
        )
        {
            return;
        }
        await onEvent({
            kind: "actionInvoked",
            item_id: itemId,
            action_id: action.id,
            invocation: confirmed ? "confirmed" : "explicit"
        });
    }
    catch (failure)
    {
        confirmationError = operationFailure(failure, "viewOperationFailed");
    }
    finally
    {
        confirming = false;
    }
}

function handleKeydown(event: KeyboardEvent): void
{
    if (event.defaultPrevented || isCompositionKey(event))
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
        if (!busy)
        {
            onBack();
        }
        return;
    }
    const verticalNavigation = (event.key === "ArrowDown" || event.key === "ArrowUp")
        && !event.shiftKey && !event.ctrlKey && !event.altKey && !event.metaKey;
    const fromDetail = list === null && event.target === document.body;
    if (event.target !== input && !fromDetail)
    {
        return;
    }
    if (verticalNavigation)
    {
        event.preventDefault();
        if (busy || !list || !totalItems)
        {
            return;
        }
        focusSearch();
        const index = selectedIndex ?? 0;
        const next = Math.max(
            0,
            Math.min(totalItems - 1, (pendingKeyboard?.index ?? index) + (event.key === "ArrowDown" ? 1 : -1))
        );
        if (next === index)
        {
            pendingKeyboard = null;
        }
        if (next !== index)
        {
            const item = itemAt(list.sections, next);
            if (item)
            {
                pendingKeyboard = null;
                selectItem(item.id);
                _revealItem(item.id);
            }
            else
            {
                pendingKeyboard = { collectionId: list.collection_id, index: next };
            }
        }
    }
    if (
        event.key === "Enter" && !busy && pendingKeyboard === null
        && !event.shiftKey && !event.ctrlKey && !event.altKey && !event.metaKey
    )
    {
        const action = primaryAction;
        if (action)
        {
            event.preventDefault();
            if (event.repeat)
            {
                return;
            }
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
    void onContextMenu(itemId, [event.clientX, event.clientY]);
}
</script>

<svelte:window
    onkeydown={handleKeydown}
    onfocus={handleFocus}
/>

<main
    class="extension-view"
    aria-label={list ? presentation.text(list.title, localization.locale) : detailTitle ?? localization.text({ message: "view-label" })}
    oncontextmenu={list ? undefined : event => _openContextMenu(null, event)}
>
    <header class="collection-header" class:has-filter={Boolean(list?.filter)}>
        <Button class="back" onclick={onBack} disabled={busy} aria-label={localization.text({ message: "view-back" })}>
            <ChevronLeftIcon size={16} />
        </Button>
        {#if list}
            <Input
                variant="search"
                bind:ref={input}
                value={query}
                role="combobox"
                aria-label={presentation.text(list.search_placeholder, localization.locale) || localization.text({ message: "view-search" })}
                placeholder={presentation.text(list.search_placeholder, localization.locale)}
                aria-autocomplete="list"
                aria-controls="extension-items"
                aria-expanded={totalItems > 0}
                aria-activedescendant={selected && items.some(item => item.id === selected.id) ? `view-item-${snapshot.routeId}-${selected.id}` : undefined}
                autocomplete="off"
                spellcheck="false"
                oninput={(event =>
                {
                    onQuery(event.currentTarget.value, _initialItemCount());
                })}
            />
            {#if list.filter}
                <div class="filter" role="group" aria-label={localization.text({ message: "view-content-filter" })}>
                    {#each list.filter.options as option (option.value)}
                        <Button
                            aria-pressed={option.value === list.filter.selected_value}
                            aria-disabled={busy}
                            onmousedown={(event => event.preventDefault())}
                            onclick={() =>
                            {
                                if (!busy && list?.filter && option.value !== list.filter.selected_value)
                                {
                                    onEvent({
                                        kind: "filterChanged",
                                        filter_id: list.filter.id,
                                        value: option.value,
                                        minimum_items: _initialItemCount()
                                    });
                                }
                            }}
                        >
                            {presentation.text(option.title, localization.locale)}
                        </Button>
                    {/each}
                </div>
            {/if}
        {:else}
            <h1>{detailTitle ?? localization.text({ message: "view-details" })}</h1>
        {/if}
    </header>
    {#if error || confirmationErrorText}<div class="error" role="alert">{error ?? confirmationErrorText}</div>{/if}
    <div class="content" class:split={list?.layout === "split"} aria-busy={busy}>
        {#if list}
            <!-- The combobox owns list navigation, including clicks on empty space. -->
            <div
                class="list-pane"
                bind:this={listContainer}
                onmousedowncapture={event =>
                {
                    if (event.target instanceof Element && event.target.closest("button"))
                    {
                        return;
                    }
                    event.preventDefault();
                    focusSearch();
                }}
            >
                <div class="row-measure" bind:this={rowMeasure} aria-hidden="true"></div>
                <div class="heading-measure" bind:this={headingMeasure} aria-hidden="true">Section</div>
                {#if totalItems > 0}
                    <ScrollArea
                        bind:viewport={listPane}
                        onscroll={event =>
                        {
                            scrollTop = event.currentTarget.scrollTop;
                            if (scrollAnchor?.physical !== scrollTop)
                            {
                                scrollAnchor = null;
                            }
                        }}
                        viewportClass="extension-viewport"
                    >
                        <ul
                            class="collection-track"
                            style:height={`${geometry.extent}px`}
                            id="extension-items"
                            role="listbox"
                            aria-label={presentation.text(list.title, localization.locale)}
                        >
                            {#each list.sections as section, sectionIndex (section.id)}
                                <li role="presentation">
                                    {#if section.title}<h2
                                            class="section-heading"
                                            style:top={`${sectionStarts[sectionIndex]!.top - coordinateShift}px`}
                                        >
                                            {presentation.text(section.title, localization.locale)}
                                        </h2>{/if}
                                    <ul
                                        class="section-window"
                                        role="group"
                                        aria-label={section.title ? presentation.text(section.title, localization.locale) : localization.text({ message: "view-items" })}
                                        style:top={`${
                                            sectionStarts[sectionIndex]!.top + (section.title ? headingHeight : 0) + section.offset * rowHeight
                                            - coordinateShift
                                        }px`}
                                    >
                                        {#each section.items as item, index (item.id)}
                                            <li
                                                id={`view-item-${snapshot.routeId}-${item.id}`}
                                                role="option"
                                                aria-posinset={sectionStarts[sectionIndex]!.index + section.offset + index + 1}
                                                aria-setsize={totalItems}
                                                aria-selected={item.id === selected?.id}
                                                aria-disabled={busy}
                                                oncontextmenu={event =>
                                                {
                                                    void _openContextMenu(item.id, event);
                                                }}
                                                onclick={() =>
                                                {
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
                                                    <span class="collection-icon" aria-hidden="true">
                                                        {#if item.icon}
                                                            {#if typeof item.icon === "string"}
                                                                <SemanticContentIcon kind={item.icon} />
                                                            {:else}
                                                                <CachedFileIcon
                                                                    reference={item.icon.native}
                                                                    {resourceOrigin}
                                                                    extensionId={snapshot.extensionId}
                                                                />
                                                            {/if}
                                                        {/if}
                                                    </span>
                                                    {#if item.subtitle}
                                                        <span class="item-copy"><span class="collection-title">{
                                                                presentation.text(item.title, localization.locale)
                                                            }</span><small>{
                                                                presentation.text(item.subtitle, localization.locale)
                                                            }</small></span>
                                                    {:else}
                                                        <span class="collection-title">{
                                                            presentation.text(item.title, localization.locale)
                                                        }</span>
                                                    {/if}
                                                </div>
                                            </li>
                                        {/each}
                                    </ul>
                                </li>
                            {/each}
                        </ul>
                        {#if items.length === 0}<span class="loading-items" role="status">{
                                localization.text({ message: "view-loading-items" })
                            }</span>{/if}
                    </ScrollArea>
                    {#if rangeRead.failed}
                        <div class="range-error" role="status">
                            <span>{localization.text({ message: "view-items-load-failed" })}</span>
                            <Button disabled={!canLoadMore} onclick={() => rangeRead.reset()}>{
                                localization.text({ message: "action-retry" })
                            }</Button>
                        </div>
                    {/if}
                {:else}
                    <div class="list-empty">
                        <EmptyState
                            title={presentation.text(list.empty_title, localization.locale)}
                            description={presentation.text(list.empty_description, localization.locale)}
                        >
                            {#snippet icon()}
                                {#if list.search_text.trim()}
                                    <SearchIcon size={48} strokeWidth={1.5} />
                                {:else}
                                    <FolderOpenIcon size={48} strokeWidth={1.5} />
                                {/if}
                            {/snippet}
                        </EmptyState>
                    </div>
                {/if}
            </div>
        {/if}
        {#if !list || list.layout === "split"}<div
                class="detail-pane"
                aria-busy={detailPending}
            >
                <ScrollArea bind:viewport={detailPane}>
                    {#if detail}
                        <ViewDetail
                            {detail}
                            presentation={detailPresentation}
                            busy={!canLoadMore || detailPending}
                            {onReadText}
                            onLanguageChange={(document, label) =>
                            {
                                detectedLanguage = { document, label };
                            }}
                            {textScope}
                            viewport={detailPane}
                            revision={snapshot.revision}
                            resourceOrigin={resourceOrigin}
                            extensionId={snapshot.extensionId}
                        />
                    {/if}
                </ScrollArea>
            </div>{/if}
    </div>
    <StatusBar
        leadingEntries={[{
            id: "extension-menu",
            title: localization.text({ message: "extension-menu-label", args: { name: extensionName } }),
            icon: { src: snapshot.extensionIconUrl },
            iconOnly: true,
            menu: { controls: "context-menu", expanded: menuOpen }
        }, ...leadingStatusEntries]}
        trailingEntries={trailingStatusEntries}
        trailingText={languageLabel}
        onInvoke={(id, trigger) =>
        {
            if (id === "extension-menu")
            {
                onMenu(trigger);
            }
            else
            {
                invokeAction(id.slice("action:".length));
            }
        }}
    />
</main>

<style>
.extension-view { display: flex; flex-direction: column; width: 100%; height: 100%; overflow: hidden; border: 0; border-radius: var(--radius-window); background: var(--surface-window); color: var(--text-primary); }
header.has-filter { grid-template-columns: var(--icon-size) minmax(0, 1fr) auto; }
h1 { margin: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-size: var(--font-search); line-height: 1.2; }
/* Pending view requests keep toolbar contrast stable; native disabled and aria-busy still expose the interaction state. */
header :global(button:disabled) { opacity: 1; }
header :global(.back) { display: grid; justify-self: center; width: 2rem; height: 2rem; place-items: center; padding: 0; background: transparent; border: 0; }
header :global(.back svg) { width: 1rem; height: 1rem; }
.content { display: flex; flex: 1; min-height: 0; }
.list-pane, .detail-pane { display: flex; min-width: 0; min-height: 0; flex: 1; overflow: hidden; }
.split .list-pane { flex: 0 1 38%; }
.split .detail-pane { flex: 1 1 62%; border-left: 1px solid var(--border-subtle); }
ul { list-style: none; margin: 0; padding: 0; }
.collection-track { position: relative; flex: none; overflow: hidden; }
.section-window, .section-heading { position: absolute; left: 0; width: 100%; }
.list-pane [role='group'] { display: grid; grid-template-columns: minmax(0, 1fr); }
.heading-measure { position: absolute; visibility: hidden; pointer-events: none; }
.row-measure { position: absolute; height: var(--row-height); width: 0; visibility: hidden; pointer-events: none; }
.list-pane { position: relative; flex-direction: column; padding: var(--space-2); overflow-x: hidden; }
:global(.extension-viewport) { overflow-anchor: none; }
.loading-items { position: absolute; inset: var(--space-5) 0 auto; text-align: center; color: var(--text-secondary); font-size: var(--font-meta); pointer-events: none; }
.range-error { position: absolute; z-index: 2; right: var(--space-2); bottom: var(--space-2); left: var(--space-2); display: flex; justify-content: space-between; align-items: center; gap: var(--space-2); padding: var(--space-2); border: 1px solid var(--border-subtle); border-radius: var(--control-radius); background: var(--surface-popup); font-size: var(--font-meta); }
h2, .heading-measure { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-size: var(--font-meta); font-weight: 500; color: var(--text-secondary); padding: var(--space-2); margin: 0; }
[role='option'] { min-width: 0; }
.option-surface { grid-template-columns: var(--icon-size) minmax(0, 1fr); }
[role='option']:hover:not([aria-disabled='true']) > .option-surface { background: var(--surface-hovered); }
[role='option'][aria-selected='true'] > .option-surface, [role='option'][aria-selected='true']:hover > .option-surface { background: var(--surface-selected); }
.item-copy { display: flex; min-width: 0; flex: 1; flex-direction: column; justify-content: center; line-height: 1.25; }
.item-copy > span, .item-copy > small { display: block; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
small { color: var(--text-secondary); font-size: var(--font-meta); margin-top: var(--space-1); }
.list-empty { display: grid; flex: 1; min-width: 0; place-items: center; }
.filter { display: flex; align-items: center; gap: calc(var(--space-1) / 2); white-space: nowrap; }
.filter :global(button) { border-color: transparent; border-radius: 999px; background: transparent; padding: 0.35rem var(--space-2); }
.filter :global(button:hover:not(:disabled, [aria-disabled="true"])) { border-color: transparent; background: var(--surface-hovered); }
.filter :global(button[aria-pressed='true']) { background: var(--surface-selected); }
.filter :global(button[aria-pressed='true']:hover) { background: var(--surface-selected); }
.error { padding: var(--space-2) var(--space-5); font-size: var(--font-meta); white-space: pre-wrap; overflow-wrap: anywhere; max-height: 35%; overflow: auto; flex-shrink: 0; }
</style>
