<script lang="ts">
import type { SearchResult } from "../types";
import ExtensionIcon from "./ExtensionIcon.svelte";

interface Props
{
    result: SearchResult;
    active: boolean;
    position: number;
    total: number;
    onActivate: () => void;
    onInvoke: () => void;
    onContextMenu: (event: MouseEvent) => void;
}

const { result, active, position, total, onActivate, onInvoke, onContextMenu }: Props = $props();
</script>

<li
    class="collection-row"
    id={`result-${position}`}
    role="option"
    aria-selected={active}
    aria-posinset={position}
    aria-setsize={total}
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
    <span class="copy" class:description={result.subtitle?.kind === "description"}>
        <span class="title" title={result.title}>{result.title}</span>
        {#if result.subtitle}
            <span class="subtitle" class:label={result.subtitle.kind === "label"} title={result.subtitle.text}>{
                result.subtitle.text
            }</span>
        {/if}
    </span>
    <span class="kind">{result.kind}</span>
</li>

<style>
li {
  height: var(--row-height);
  flex-shrink: 0;
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
.subtitle {
  overflow: hidden;
  text-overflow: ellipsis;
  white-space: nowrap;
}

.title {
  min-width: 0;
  font-size: var(--font-row);
  font-weight: 500;
}

.subtitle {
  min-width: 0;
}

.subtitle.label,
.kind {
  flex-shrink: 0;
  overflow: visible;
  white-space: nowrap;
}

.description .title {
  flex-shrink: 0;
  max-width: 100%;
}

.description .subtitle {
  flex: 0 1 auto;
}

.subtitle,
.kind {
  color: var(--text-secondary);
  font-size: var(--font-meta);
}
</style>
