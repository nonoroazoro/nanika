<script lang="ts">
  import type { SearchResult } from '../types'

  interface Props {
    result: SearchResult
    active: boolean
    onActivate: () => void
    onInvoke: () => void
  }

  let { result, active, onActivate, onInvoke }: Props = $props()
</script>

<li
  id={`result-${result.extensionId}-${result.entryId}`}
  role="option"
  aria-selected={active}
  class:active
  onpointermove={onActivate}
  onmousedown={(event) => event.preventDefault()}
  onclick={onInvoke}
  onkeydown={(event) => {
    if (event.key === 'Enter' || event.key === ' ') {
      event.preventDefault()
      onInvoke()
    }
  }}
  tabindex="-1"
>
  <span class="icon" aria-hidden="true">
    {#if result.iconUrl}
      <img src={result.iconUrl} alt="" />
    {:else}
      <span class="fallback">{result.title.slice(0, 1)}</span>
    {/if}
  </span>
  <span class="copy">
    <span class="title">{result.title}</span>
    {#if result.subtitle}
      <span class="subtitle">{result.subtitle}</span>
    {/if}
  </span>
  <span class="kind">{result.kind}</span>
</li>

<style>
  li {
    display: grid;
    grid-template-columns: var(--icon-size) minmax(0, 1fr) auto;
    align-items: center;
    min-height: var(--row-height);
    gap: var(--space-3);
    padding: 0 var(--space-3);
    border-radius: var(--radius-row);
    color: var(--text-primary);
    cursor: default;
  }

  li.active {
    background: var(--surface-selected);
  }

  .icon,
  img,
  .fallback {
    width: var(--icon-size);
    height: var(--icon-size);
  }

  img {
    display: block;
    object-fit: contain;
  }

  .fallback {
    display: grid;
    place-items: center;
    border-radius: var(--radius-icon);
    background: var(--surface-raised);
    color: var(--text-secondary);
    font-size: 0.75rem;
    font-weight: 600;
  }

  .copy {
    display: flex;
    min-width: 0;
    align-items: baseline;
    gap: var(--space-2);
  }

  .title,
  .subtitle,
  .kind {
    overflow: hidden;
    text-overflow: ellipsis;
    white-space: nowrap;
  }

  .title {
    font-size: var(--font-row);
    font-weight: 500;
  }

  .subtitle,
  .kind {
    color: var(--text-secondary);
    font-size: var(--font-meta);
  }
</style>
