<script lang="ts">
import type { SearchResult } from "../types";
import CommandIconTile from "./CommandIconTile.svelte";

interface Props
{
    result: SearchResult;
    active: boolean;
    onActivate: () => void;
    onInvoke: () => void;
}

const { result, active, onActivate, onInvoke }: Props = $props();
let iconFailed = $state(false);
</script>

<li
    class="collection-row"
    id={`result-${result.extensionId}-${result.entryId}`}
    role="option"
    aria-selected={active}
    class:active
    onpointermove={onActivate}
    onmousedown={(event => event.preventDefault())}
    onclick={onInvoke}
    onkeydown={(event =>
    {
        if (event.key === "Enter" || event.key === " ")
        {
            event.preventDefault();
            onInvoke();
        }
    })}
    tabindex="-1"
>
    <span class="icon" aria-hidden="true">
        {#if result.commandIcon}
            <CommandIconTile kind={result.commandIcon} />
        {:else}
            <span class="fallback">
                <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="1.5">
                    <rect x="5" y="4" width="14" height="16" rx="3" />
                    <path d="M9 9h6M9 12h6M9 15h3" />
                </svg>
            </span>
        {/if}
        {#if result.iconUrl}
            <img
                src={result.iconUrl}
                alt=""
                hidden={iconFailed}
                onerror={() =>
                {
                    iconFailed = true;
                }}
                onload={() =>
                {
                    iconFailed = false;
                }}
            />
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

.icon {
  display: grid;
  position: relative;
  place-items: center;
}

img {
  position: absolute;
  inset: 0;
  display: block;
  object-fit: contain;
}

img[hidden] {
  display: none;
}

.fallback {
  display: grid;
  place-items: center;
  border-radius: var(--radius-icon);
  background: var(--surface-raised);
  color: var(--text-secondary);
}

.fallback svg {
  width: 1.25rem;
  height: 1.25rem;
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
