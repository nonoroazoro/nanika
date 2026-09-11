<script lang="ts">
import { onMount } from "svelte";
import type { ExtensionViewSnapshot, ViewEvent } from "../types";
import ViewDetail from "./ViewDetail.svelte";

function itemIcon(subtitle: string | null | undefined): string
{
    const value = subtitle?.toLowerCase() ?? "";
    if (value.includes("image"))
    {
        return "▧";
    }
    if (value.includes("file"))
    {
        return "▰";
    }
    return "▤";
}

const { snapshot, busy, error, onQuery, onEvent, onBack }: {
    snapshot: ExtensionViewSnapshot;
    busy: boolean;
    error: string | null;
    onQuery: (text: string) => void;
    onEvent: (event: ViewEvent) => void;
    onBack: () => void;
} = $props();
const list = $derived(snapshot.view.kind === "list" ? snapshot.view.list : null);
const detail = $derived(snapshot.view.kind === "detail" ? snapshot.view.detail : list?.detail ?? null);
const items = $derived(list?.sections.flatMap(section => section.items) ?? []);
const selected = $derived(items.find(item => item.id === list?.selected_item_id) ?? null);
const actions = $derived(list ? selected?.actions ?? [] : detail?.actions ?? []);
let query = $state("");
let input = $state<HTMLInputElement>();
let options = $state<HTMLUListElement>();
let surface: HTMLElement;

onMount(() =>
{
    query = list?.search_text ?? "";
    focusSearch();
});

function focusSearch(): void
{
    if (input)
    {
        input.focus({ preventScroll: true });
    }
    else
    {
        surface.focus({ preventScroll: true });
    }
}

function activateItem(item: typeof items[number]): void
{
    const primary = item.actions.find(action => action.style === "primary");
    if (primary)
    {
        onEvent({ kind: "actionInvoked", item_id: item.id, action_id: primary.id });
    }
}

function handleKeydown(event: KeyboardEvent): void
{
    if (event.isComposing || event.target instanceof HTMLSelectElement)
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
    if (event.target !== input && event.target !== surface)
    {
        return;
    }
    if (event.key === "ArrowDown" || event.key === "ArrowUp")
    {
        event.preventDefault();
        if (busy || !list || !items.length)
        {
            return;
        }
        const index = items.findIndex(item => item.id === list.selected_item_id);
        const next = Math.max(0, Math.min(items.length - 1, index + (event.key === "ArrowDown" ? 1 : -1)));
        const item = items[next];
        if (item)
        {
            if (item.id === list.selected_item_id)
            {
                return;
            }
            options?.querySelectorAll<HTMLElement>('[role="option"]').item(next)?.scrollIntoView({
                block: "nearest",
                inline: "nearest",
                behavior: "instant"
            });
            onEvent({ kind: "selectionChanged", item_id: item.id });
        }
    }
    if (event.key === "Enter" && !busy)
    {
        const action = actions.find(candidate => candidate.style === "primary");
        if (action)
        {
            event.preventDefault();
            onEvent({ kind: "actionInvoked", item_id: selected?.id ?? null, action_id: action.id });
        }
    }
}
</script>

<svelte:window onkeydown={handleKeydown} />

<section
    class="extension-view"
    bind:this={surface}
    tabindex="-1"
    aria-label={list?.title ?? detail?.title ?? "Extension view"}
>
    <header>
        <button class="back" type="button" onclick={onBack} disabled={busy} aria-label="Back to previous view">
            ‹
        </button>
        <div><h1>{list?.title ?? detail?.title ?? "Details"}</h1></div>
    </header>
    {#if list}
        <div class="search">
            <input
                bind:this={input}
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
                oninput={(event => onQuery(event.currentTarget.value))}
            />
            {#if list.filter}
                <select
                    aria-label="Content filter"
                    value={list.filter.selected_value}
                    disabled={busy}
                    onchange={(event =>
                    {
                        if (list?.filter)
                        {
                            onEvent({ kind: "filterChanged", filter_id: list.filter.id, value: event.currentTarget.value });
                        }
                    })}
                >
                    {#each list.filter.options as option (option.value)}<option value={option.value}>
                            {option.title}
                        </option>{/each}
                </select>
            {/if}
        </div>
    {/if}
    {#if error}<div class="error" role="alert">{error}</div>{/if}
    <div class="content" class:split={list?.layout === "split"} aria-busy={busy}>
        {#if list}
            <div class="list-pane">
                <ul bind:this={options} id="extension-items" role="listbox" aria-label={list.title}>
                    {#each list.sections as section (section.id)}
                        <li role="presentation">
                            {#if section.title}<h2>{section.title}</h2>{/if}
                            <ul role="group" aria-label={section.title ?? "Items"}>
                                {#each section.items as item (item.id)}
                                    <li
                                        id={`view-item-${snapshot.routeId}-${item.id}`}
                                        role="option"
                                        aria-selected={item.id === list.selected_item_id}
                                        aria-disabled={busy}
                                        tabindex="-1"
                                        onmousedown={(event => event.preventDefault())}
                                        onclick={() =>
                                        {
                                            if (item.id !== list.selected_item_id)
                                            {
                                                onEvent({ kind: "selectionChanged", item_id: item.id });
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
                                        <span class="item-icon" aria-hidden="true">{itemIcon(item.subtitle)}</span>
                                        <span class="item-copy"><span>{item.title}</span>{#if item.subtitle}<small>{
                                                    item.subtitle
                                                }</small>{/if}</span>
                                    </li>
                                {/each}
                            </ul>
                        </li>
                    {/each}
                </ul>
                {#if !items.length}<p class="empty">No items</p>{/if}
                {#if list.next_cursor}<button
                        class="more"
                        type="button"
                        disabled={busy}
                        onclick={() =>
                        {
                            if (list?.next_cursor)
                            {
                                onEvent({ kind: "loadMore", cursor: list.next_cursor });
                            }
                        }}
                    >
                        Load more
                    </button>{/if}
            </div>
        {/if}
        {#if detail && (!list || list.layout === "split")}<div class="detail-pane"><ViewDetail {detail} /></div>{/if}
    </div>
    <footer>
        {#each actions as action (action.id)}
            <button
                type="button"
                class:primary={action.style === "primary"}
                disabled={busy}
                onclick={() => onEvent({ kind: "actionInvoked", item_id: selected?.id ?? null, action_id: action.id })}
            >
                {action.title}
            </button>
        {/each}
    </footer>
</section>

<style>
.extension-view { display: flex; flex-direction: column; width: 100%; height: 100%; overflow: hidden; border: 1px solid var(--border-window); border-radius: var(--radius-window); background: var(--surface-window); box-shadow: var(--shadow-window); color: var(--text-primary); }
header { display: flex; gap: var(--space-3); align-items: center; padding: var(--space-3) var(--space-5); border-bottom: 1px solid var(--border-subtle); }
h1 { margin: 0; font-size: var(--font-row); }
button, select { border: 1px solid var(--border-window); border-radius: var(--radius-row); background: var(--surface-raised); color: inherit; padding: 0.4rem 0.75rem; font-size: var(--font-meta); }
button:disabled, select:disabled { opacity: 0.5; }
button:focus-visible, select:focus-visible { outline: 2px solid var(--accent); outline-offset: 2px; }
.back { font-size: 1.5rem; background: transparent; border: 0; }
.search { display: grid; grid-template-columns: minmax(0, 1fr) auto; align-items: center; gap: var(--space-3); min-height: var(--search-height); padding: 0 var(--space-5); border-bottom: 1px solid var(--border-subtle); }
input { min-width: 0; width: 100%; height: 100%; border: 0; outline: 0; background: transparent; color: inherit; padding: 0; font: inherit; font-size: var(--font-search); caret-color: var(--accent); }
input::placeholder { color: var(--text-tertiary); opacity: 1; }
.content { display: flex; flex: 1; min-height: 0; }
.list-pane, .detail-pane { min-width: 0; flex: 1; overflow: auto; }
.split .detail-pane { border-left: 1px solid var(--border-subtle); }
ul { list-style: none; margin: 0; padding: 0; }
.list-pane { padding: var(--space-2); }
h2 { font-size: var(--font-meta); font-weight: 500; color: var(--text-secondary); padding: var(--space-2); margin: 0; }
[role='option'] { display: flex; align-items: center; gap: var(--space-3); padding: var(--space-3); border-radius: var(--radius-row); cursor: default; }
[role='option'][aria-selected='true'] { background: var(--surface-selected); }
.item-icon { display: grid; flex: 0 0 2rem; width: 2rem; height: 2rem; place-items: center; border-radius: var(--radius-row); background: var(--surface-raised); color: var(--text-secondary); font-size: 1.1rem; }
.item-copy { min-width: 0; flex: 1; }
.item-copy > span, small { display: block; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
small { color: var(--text-secondary); font-size: var(--font-meta); margin-top: var(--space-1); }
.empty { color: var(--text-secondary); text-align: center; padding: var(--space-5); }
.more { display: block; margin: var(--space-3) auto; }
footer { display: flex; justify-content: flex-end; gap: var(--space-2); min-height: 3rem; padding: var(--space-2) var(--space-5); border-top: 1px solid var(--border-subtle); }
.primary { font-weight: 600; }
.error { padding: var(--space-2) var(--space-5); font-size: var(--font-meta); }
</style>
