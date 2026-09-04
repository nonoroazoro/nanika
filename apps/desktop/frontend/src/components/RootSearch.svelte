<script lang="ts">
  import { onMount } from 'svelte'

  import type { RootSearchSnapshot, SearchResult } from '../types'
  import { clampIndex } from '../logic'
  import ResultRow from './ResultRow.svelte'

  interface Props {
    snapshot: RootSearchSnapshot
    onQuery: (query: string) => void
    onDismiss: () => void
    onInvoke: (result: SearchResult) => void
  }

  let { snapshot, onQuery, onDismiss, onInvoke }: Props = $props()
  let query = $state('')
  let requestedActiveIndex = $state(0)
  let input: HTMLInputElement

  const activeIndex = $derived(
    snapshot.results.length === 0 ? -1 : clampIndex(requestedActiveIndex, snapshot.results.length),
  )
  const activeResult = $derived(snapshot.results[activeIndex] ?? null)
  const activeId = $derived(
    activeResult ? `result-${activeResult.extensionId}-${activeResult.entryId}` : undefined,
  )

  onMount(() => {
    query = snapshot.query
    input.focus()
    if (query.length > 0) {
      input.select()
    }
  })

  function handleKeydown(event: KeyboardEvent): void {
    if (event.key === 'ArrowDown') {
      event.preventDefault()
      requestedActiveIndex = clampIndex(activeIndex + 1, snapshot.results.length)
      return
    }
    if (event.key === 'ArrowUp') {
      event.preventDefault()
      requestedActiveIndex = clampIndex(activeIndex - 1, snapshot.results.length)
      return
    }
    if (event.key === 'Enter' && activeResult) {
      event.preventDefault()
      onInvoke(activeResult)
      return
    }
    if (event.key === 'Escape') {
      event.preventDefault()
      onDismiss()
    }
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
      aria-controls="root-results"
      aria-expanded={snapshot.results.length > 0}
      aria-activedescendant={activeId}
      autocomplete="off"
      spellcheck="false"
      placeholder="Search apps and commands"
      oninput={() => onQuery(query)}
      onkeydown={handleKeydown}
    />
  </div>

  <section class="results" aria-label="Results">
    {#if snapshot.results.length > 0}
      <ul id="root-results" role="listbox">
        {#each snapshot.results as result, index (`${result.extensionId}:${result.entryId}`)}
          <ResultRow
            {result}
            active={index === activeIndex}
            onActivate={() => (requestedActiveIndex = index)}
            onInvoke={() => onInvoke(result)}
          />
        {/each}
      </ul>
    {:else if snapshot.complete}
      <div class="empty" role="status">
        <span>No results</span>
        <small>Enable an extension or try another search.</small>
      </div>
    {:else}
      <div class="empty" role="status">Loading extensions...</div>
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
    min-height: 0;
    padding: var(--space-2);
  }

  ul {
    height: 100%;
    margin: 0;
    padding: 0;
    overflow: auto;
    list-style: none;
    scrollbar-width: thin;
  }

  .empty {
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
</style>
