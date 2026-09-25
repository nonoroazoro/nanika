<script lang="ts">
import type { SearchResult } from "../types";
import ExtensionIcon from "./ExtensionIcon.svelte";

interface Props
{
    result: SearchResult;
    active: boolean;
    onActivate: () => void;
    onInvoke: () => void;
    onContextMenu: (event: MouseEvent) => void;
}

const { result, active, onActivate, onInvoke, onContextMenu }: Props = $props();
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
    oncontextmenu={onContextMenu}
    onkeydown={(event =>
    {
        if (event.key === "Enter" || event.key === " ")
        {
            event.preventDefault();
            onInvoke();
        }
    })}
>
    <span class="icon" aria-hidden="true"><ExtensionIcon src={result.iconUrl} /></span>
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

.icon { display: grid; width: var(--icon-size); height: var(--icon-size); place-items: center; }

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
