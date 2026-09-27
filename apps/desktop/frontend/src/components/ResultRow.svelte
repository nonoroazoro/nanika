<script lang="ts">
import type { SearchResult } from "../types";
import ExtensionIcon from "./ExtensionIcon.svelte";
import ActionSymbol from "./ActionSymbol.svelte";
import ShortcutKeys from "./ShortcutKeys.svelte";

interface Props
{
    result: SearchResult;
    active: boolean;
    confirmationTitle?: string | null;
    position: number;
    total: number;
    onActivate: () => void;
    onInvoke: () => void;
    onContextMenu: (event: MouseEvent) => void;
}

const { result, active, confirmationTitle = null, position, total, onActivate, onInvoke, onContextMenu }: Props =
    $props();
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
        if (event.ctrlKey || event.altKey || event.metaKey || event.shiftKey)
        {
            return;
        }
        if (event.key === "Enter" || event.key === " ")
        {
            event.preventDefault();
            if (!event.repeat && !event.isComposing)
            {
                onInvoke();
            }
        }
    })}
>
    <span class="icon" aria-hidden="true">
        {#if result.icon?.kind === "symbol"}
            <span class="result-symbol"><ActionSymbol name={result.icon.name} /></span>
        {:else}
            <ExtensionIcon src={result.icon?.url ?? null} />
        {/if}
    </span>
    <span
        class="copy"
        class:confirming={Boolean(confirmationTitle)}
        class:description={!confirmationTitle && result.subtitle?.kind === "description"}
    >
        <span class="title" title={result.title}>{result.title}</span>
        {#if confirmationTitle}
            <span class="confirmation" title={confirmationTitle}>
                <span class="confirmation-label">{confirmationTitle}</span>
                <span class="confirmation-key"><ShortcutKeys keys={["↵"]} /></span>
            </span>
        {:else if result.subtitle}
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

.result-symbol { display: block; width: 100%; height: 100%; color: var(--text-secondary); }

.copy {
  display: flex;
  min-width: 0;
  /* The confirmation keycap must not shift text by changing the shared baseline group. */
  align-items: center;
  gap: var(--space-2);
}

.title,
.subtitle,
.confirmation-label {
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

.confirming .title { max-width: 45%; }
.confirmation { display: inline-flex; min-width: 0; align-items: center; gap: var(--space-2); color: var(--text-danger); font-size: var(--font-meta); font-weight: 500; }
.confirmation-key { flex: 0 0 auto; }

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
