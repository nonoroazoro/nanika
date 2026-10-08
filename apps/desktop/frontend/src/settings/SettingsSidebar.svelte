<script lang="ts">
import { Tooltip } from "bits-ui";
import TruncatedButton from "../components/ui/TruncatedButton.svelte";
import Button from "../components/ui/Button.svelte";
import EmptyState from "../components/ui/EmptyState.svelte";
import DismissButton from "../components/ui/DismissButton.svelte";
import SearchIcon from "../components/icons/SearchIcon.svelte";
import InfoIcon from "../components/icons/InfoIcon.svelte";
import Input from "../components/ui/Input.svelte";
import ShortcutKeys from "../components/ui/ShortcutKeys.svelte";
import { styleMotion } from "../components/motion/style";
import ScrollArea from "../components/ui/ScrollArea.svelte";
import SelectionHighlight from "../components/motion/SelectionHighlight.svelte";
import ExtensionIcon from "../components/extensions/ExtensionIcon.svelte";
import type { ExtensionSettings } from "../generated/ExtensionSettings";
import type { SettingsSearchEntry } from "../generated/SettingsSearchEntry";
import type { SettingsSearchTarget } from "../generated/SettingsSearchTarget";
import type { SettingsSearchState } from "./SettingsSearchState.svelte";
import { settingsAnchor } from "./anchor";
import { groupSearchResults } from "./groupSearchResults";

let { extensions, selection, revealed, search, input = $bindable(), searchShortcut, onNavigate, onQuery }: {
    extensions: ExtensionSettings[];
    selection: string;
    revealed: string | null;
    search: SettingsSearchState;
    input?: HTMLInputElement;
    searchShortcut: string;
    onNavigate: (page: string, target: SettingsSearchTarget) => void;
    onQuery: (query: string) => void;
} = $props();
const shortcutMotion = styleMotion({ properties: ["opacity"], scope: ".search-field" });
let composing = false;
let navigation = $state<HTMLElement | null>(null);
let selectedButton = $state<HTMLElement | null>(null);
// Keep navigation until the first completed search, just as later queries retain their results.
const showingResults = $derived(search.query.trim().length > 0 && (search.hasCompletedSearch || search.error !== null));
const groups = $derived(
    groupSearchResults(search.results, ["general", ...extensions.map(extension => extension.id), "about"])
);

// Resolve the selected DOM destination after the keyed navigation has been rendered.
$effect(() =>
{
    const selectable = showingResults
        ? revealed !== null && groups.some(group => group.pageId === selection)
        : selection === "general" || selection === "about" || extensions.some(extension => extension.id === selection);
    selectedButton = selectable ? navigation?.querySelector<HTMLElement>("button.active") ?? null : null;
});

function _resultSelected(entry: SettingsSearchEntry): boolean
{
    return entry.pageId === selection && settingsAnchor(entry.pageId, entry.target) === revealed;
}
</script>

<div class="search-field" data-state={search.query ? "filled" : "empty"}>
    <SearchIcon size={16} strokeWidth={1.75} />
    <Input
        bind:ref={input}
        variant="embedded"
        value={search.query}
        aria-label="Search settings"
        aria-keyshortcuts={searchShortcut}
        placeholder="Search"
        autocomplete="off"
        spellcheck={false}
        oninput={event =>
        {
            if (!composing)
            {
                onQuery(event.currentTarget.value);
            }
        }}
        oncompositionstart={() =>
        {
            composing = true;
        }}
        oncompositionend={event =>
        {
            composing = false;
            onQuery(event.currentTarget.value);
        }}
        onkeydown={event =>
        {
            if (!event.isComposing && event.key === "ArrowDown")
            {
                event.preventDefault();
                navigation?.querySelector<HTMLButtonElement>("button")?.focus();
            }
        }}
    />
    <span class="search-accessory">
        <span class="search-shortcut" {@attach shortcutMotion}>
            <ShortcutKeys keys={searchShortcut === "Meta+F" ? ["⌘", "F"] : ["Ctrl", "F"]} />
        </span>
        {#if search.query}
            <DismissButton
                label="Clear search"
                onmousedown={event => event.preventDefault()}
                onclick={() =>
                {
                    onQuery("");
                    input?.focus({ preventScroll: true });
                }}
            />
        {/if}
    </span>
</div>
<Tooltip.Provider>
    <ScrollArea>
        <nav
            bind:this={navigation}
            aria-label={showingResults ? "Settings search results" : "Settings sections"}
            aria-busy={search.busy}
        >
            {#if showingResults}
                {#if search.error}
                    <p class="search-message" role="alert">{search.error}</p>
                    <Button onclick={() => search.refresh()}>Try again</Button>
                {:else if search.results.length === 0}
                    <EmptyState title="No settings found" description="Try a different search.">
                        {#snippet icon()}<SearchIcon size={48} strokeWidth={1.5} />{/snippet}
                    </EmptyState>
                {:else}
                    <span class="sr-only" role="status">{search.results.length} results</span>
                    {#each groups as group (group.pageId)}
                        <section class="result-group" aria-label={group.title}>
                            <h2>
                                <TruncatedButton
                                    label={group.title}
                                    class={{ active: revealed === settingsAnchor(group.pageId, { kind: "page" }) }}
                                    aria-disabled={search.busy || undefined}
                                    onclick={() =>
                                    {
                                        if (!search.busy)
                                        {
                                            onNavigate(group.pageId, { kind: "page" });
                                        }
                                    }}
                                >
                                    {#snippet icon()}{@render pageIcon(group.pageId)}{/snippet}
                                </TruncatedButton>
                            </h2>
                            {#each group.entries as entry (settingsAnchor(entry.pageId, entry.target))}
                                <TruncatedButton
                                    label={entry.title}
                                    class={["result-field", { active: _resultSelected(entry) }]}
                                    aria-current={_resultSelected(entry) ? "location" : undefined}
                                    aria-disabled={search.busy || undefined}
                                    onclick={() =>
                                    {
                                        if (!search.busy)
                                        {
                                            onNavigate(entry.pageId, entry.target);
                                        }
                                    }}
                                />
                            {/each}
                        </section>
                    {/each}
                {/if}
            {:else}
                <Button
                    class={{ active: selection === "general" }}
                    aria-current={selection === "general" ? "page" : undefined}
                    onclick={() => onNavigate("general", { kind: "page" })}
                >
                    {@render pageIcon("general")}General
                </Button>
                <h2 class="nav-heading">Extensions</h2>
                {#each extensions as extension (extension.id)}
                    <TruncatedButton
                        label={extension.name}
                        class={{ active: selection === extension.id }}
                        aria-current={selection === extension.id ? "page" : undefined}
                        onclick={() => onNavigate(extension.id, { kind: "page" })}
                    >
                        {#snippet icon()}{@render pageIcon(extension.id)}{/snippet}
                    </TruncatedButton>
                {/each}
                <div class="nav-group-start">
                    <Button
                        class={{ active: selection === "about" }}
                        aria-current={selection === "about" ? "page" : undefined}
                        onclick={() => onNavigate("about", { kind: "page" })}
                    >
                        {@render pageIcon("about")}About
                    </Button>
                </div>
            {/if}
            <SelectionHighlight container={navigation} target={selectedButton} />
        </nav>
    </ScrollArea>
</Tooltip.Provider>

{#snippet pageIcon(page: string)}
    <span class="nav-icon" aria-hidden="true">
        {#if page === "general"}
            <svg width="20" height="20" viewBox="0 0 20 20" fill="none" stroke="currentColor" stroke-width="1.25">
                <rect x="1.5" y="1.5" width="17" height="17" rx="3" />
                <path d="M1.5 6.5h17M7 6.5v12" />
            </svg>
        {:else if page === "about"}
            <InfoIcon size={20} strokeWidth={1.5} />
        {:else}
            <ExtensionIcon src={extensions.find(extension => extension.id === page)?.iconUrl ?? ""} />
        {/if}
    </span>
{/snippet}

<style>
.search-field { display: grid; grid-template-columns: 16px minmax(0, 1fr) auto; flex-shrink: 0; align-items: center; gap: 8px; min-width: 0; height: 36px; margin: 0 4px; padding: 0 8px; border-radius: var(--control-radius); background: var(--surface-selected); color: var(--text-secondary); font-size: 14px; }
.search-field :global(.ui-input) { height: 100%; min-height: 0; padding: 0; font: inherit; }
/* The hidden shortcut keeps the accessory track measured in every input state. */
.search-accessory { display: grid; min-width: 24px; height: 24px; align-items: center; justify-items: end; }
.search-accessory > :global(*) { grid-area: 1 / 1; }
.search-shortcut { opacity: 0.7; pointer-events: none; user-select: none; }
.search-field:focus-within .search-shortcut, .search-field[data-state="filled"] .search-shortcut { opacity: 0; }
nav { position: relative; isolation: isolate; display: flex; flex: 1; min-height: 0; flex-direction: column; gap: 2px; }
nav :global(button) { position: relative; z-index: 2; justify-content: flex-start; gap: var(--space-2); flex: 0 0 auto; width: 100%; min-height: var(--settings-nav-height); border: 0; border-radius: var(--control-radius); padding: 6px 10px; background: transparent; color: var(--text-secondary); text-align: left; font-size: var(--font-control); line-height: 20px; }
/* Hover uses foreground contrast; only SelectionHighlight paints a selection surface. */
nav :global(button:hover:not(:disabled, [aria-disabled="true"])), nav :global(button:focus-visible), nav :global(button.active) { background: transparent; color: var(--text-primary); }
.nav-icon { --icon-size: var(--settings-nav-icon-size); display: grid; width: var(--settings-nav-icon-size); height: var(--settings-nav-icon-size); flex-shrink: 0; place-items: center; color: inherit; }
.nav-heading { display: flex; justify-content: space-between; margin: var(--space-5) 10px var(--space-2); color: var(--text-secondary); font-size: 12px; font-weight: var(--settings-heading-weight); }
.nav-group-start { margin-top: var(--space-5); }
.result-group + .result-group { margin-top: var(--space-3); }
.result-group h2 { margin: 0; font: inherit; }
.result-group :global(.result-field) { padding-left: calc(10px + var(--settings-nav-icon-size) + var(--space-2)); }
.search-message { padding: 0 10px; color: var(--text-secondary); overflow-wrap: anywhere; }
.sr-only { position: absolute; width: 1px; height: 1px; overflow: hidden; clip-path: inset(50%); }
</style>
