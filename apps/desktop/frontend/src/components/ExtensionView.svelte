<script lang="ts">
import { onMount } from "svelte";
import type { ExtensionViewSnapshot, ViewEvent } from "../types";
import ViewDetail from "./ViewDetail.svelte";

function itemIcon(subtitle: string | null | undefined): string
{
    const value = subtitle?.toLowerCase() ?? "";
    if (value.includes("image"))
    {
        return "image";
    }
    if (value.includes("file"))
    {
        return "files";
    }
    return "text";
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
                <div class="filter" role="group" aria-label="Content filter">
                    {#each list.filter.options as option (option.value)}
                        <button
                            type="button"
                            class:active={option.value === list.filter.selected_value}
                            aria-pressed={option.value === list.filter.selected_value}
                            disabled={busy}
                            onclick={() =>
                            {
                                if (list?.filter && option.value !== list.filter.selected_value)
                                {
                                    onEvent({ kind: "filterChanged", filter_id: list.filter.id, value: option.value });
                                }
                            }}
                        >
                            {option.title}
                        </button>
                    {/each}
                </div>
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
                                    {@const icon = itemIcon(item.subtitle)}
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
                                        <span class="item-icon" aria-hidden="true">
                                            {#if icon === "image"}
                                                <svg
                                                    viewBox="0 0 24 24"
                                                    fill="none"
                                                    stroke="currentColor"
                                                    stroke-width="1.6"
                                                >
                                                    <rect x="3.5" y="4" width="17" height="16" rx="2" />
                                                    <circle cx="8.5" cy="9" r="1.5" />
                                                    <path d="m5.5 17 4.5-4 3 2.5 2-2 3.5 3.5" />
                                                </svg>
                                            {:else if icon === "files"}
                                                <svg
                                                    viewBox="0 0 24 24"
                                                    fill="none"
                                                    stroke="currentColor"
                                                    stroke-width="1.6"
                                                >
                                                    <path d="M7 3.5h7l3 3V20.5H7z" />
                                                    <path d="M14 3.5v3h3M9.5 11h5M9.5 14h5M9.5 17h3" />
                                                </svg>
                                            {:else}
                                                <svg
                                                    viewBox="0 0 24 24"
                                                    fill="none"
                                                    stroke="currentColor"
                                                    stroke-width="1.6"
                                                >
                                                    <rect x="4" y="4" width="16" height="16" rx="2" />
                                                    <path d="M8 9h8M8 12h8M8 15h5" />
                                                </svg>
                                            {/if}
                                        </span>
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
                class:destructive={action.style === "destructive"}
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
h1 { margin: 0; font-size: var(--font-row); line-height: 1.2; }
button { color: inherit; font-size: var(--font-meta); }
.back { display: grid; width: 2rem; height: 2rem; place-items: center; padding: 0; background: transparent; border: 0; }
.back svg { width: 1.1rem; height: 1.1rem; }
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
.item-icon svg { width: 1.2rem; height: 1.2rem; }
.item-copy { display: flex; min-width: 0; flex: 1; flex-direction: column; justify-content: center; line-height: 1.25; }
.item-copy > span, small { display: block; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
small { color: var(--text-secondary); font-size: var(--font-meta); margin-top: var(--space-1); }
.empty { color: var(--text-secondary); text-align: center; padding: var(--space-5); }
.more { display: block; margin: var(--space-3) auto; }
.filter { display: flex; align-items: center; gap: var(--space-1); white-space: nowrap; }
.filter button { border-color: transparent; border-radius: 999px; padding: 0.35rem 0.65rem; }
.filter button:disabled { opacity: 1; }
.filter button:hover:not(:disabled) { border-color: var(--border-window); background: var(--surface-selected); }
.filter button.active { border-color: var(--accent); background: var(--surface-selected); }
.filter button.active:hover:not(:disabled) { background: var(--surface-selected); }
footer { display: flex; justify-content: flex-end; gap: var(--space-2); min-height: 3rem; padding: var(--space-2) var(--space-5); border-top: 1px solid var(--border-subtle); }
.primary { border-color: var(--accent); background: var(--accent); color: var(--accent-foreground); font-weight: 600; }
.primary:hover:not(:disabled) { border-color: var(--accent); background: color-mix(in srgb, var(--accent) 88%, black); }
.destructive { margin-right: auto; border-color: var(--border-danger); background: transparent; color: var(--text-danger); font-weight: 500; }
.destructive:hover:not(:disabled) { border-color: var(--border-danger-hover); background: var(--surface-danger-hover); }
.error { padding: var(--space-2) var(--space-5); font-size: var(--font-meta); }
</style>
