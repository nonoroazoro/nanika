<script lang="ts">
import { isCompositionKey } from "../../ui/keyboard";
import type { SearchResult } from "../../generated/SearchResult";
import type { TargetPlatform } from "../../generated/TargetPlatform";
import ExtensionIcon from "../extensions/ExtensionIcon.svelte";
import ActionSymbol from "../extensions/ActionSymbol.svelte";

interface Props
{
    platform: TargetPlatform | null;
    result: SearchResult;
    active: boolean;
    position: number;
    total: number;
    onInvoke: () => void;
    onContextMenu: (event: MouseEvent) => void;
}

const { platform, result, active, position, total, onInvoke, onContextMenu }: Props = $props();
</script>

<li
    class="collection-row"
    id={`result-${position}`}
    role="option"
    aria-selected={active}
    aria-posinset={position}
    aria-setsize={total}
    class:active
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
            if (!event.repeat && !isCompositionKey(event))
            {
                onInvoke();
            }
        }
    })}
>
    <span class="collection-icon" aria-hidden="true">
        {#if result.icon?.kind === "symbol"}
            {#if platform}<ActionSymbol {platform} name={result.icon.name} />{/if}
        {:else}
            <ExtensionIcon src={result.icon?.url ?? null} />
        {/if}
    </span>
    <span
        class="copy"
        class:description={result.subtitle?.kind === "description"}
    >
        <span class="title collection-title" title={result.title}>{result.title}</span>
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
  grid-template-columns: var(--icon-size) minmax(0, 1fr) auto;
}

li:hover:not(.active) {
  background: var(--surface-hovered);
}

li.active {
  background: var(--surface-selected);
}
.copy {
  display: flex;
  min-width: 0;
  align-items: center;
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
