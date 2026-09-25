<script lang="ts">
import Button from "../components/Button.svelte";
import type { SettingsWindowAction } from "../types/SettingsWindowAction";

const { maximized, onAction }: {
    maximized: boolean;
    onAction: (action: SettingsWindowAction) => void;
} = $props();

function _drag(event: MouseEvent): void
{
    if (event.button === 0 && event.detail === 1)
    {
        onAction({ kind: "drag" });
    }
}
</script>

<header class="titlebar">
    <div
        class="drag-region"
        role="presentation"
        onmousedown={_drag}
        ondblclick={() => onAction({ kind: "toggleMaximize" })}
    >
        <span>Settings</span>
    </div>
    <div class="window-controls" role="group" aria-label="Window controls">
        <Button aria-label="Minimize" onclick={() => onAction({ kind: "minimize" })}>
            <svg viewBox="0 0 16 16" aria-hidden="true"><path d="M3 8h10" /></svg>
        </Button>
        <Button
            aria-label={maximized ? "Restore" : "Maximize"}
            onclick={() => onAction({ kind: "toggleMaximize" })}
        >
            <svg viewBox="0 0 16 16" aria-hidden="true">
                {#if maximized}<path d="M5 5V3h8v8h-2M3 5h8v8H3Z" />{:else}<path d="M3 3h10v10H3Z" />{/if}
            </svg>
        </Button>
        <Button class="close-window" aria-label="Close Settings" onclick={() => onAction({ kind: "close" })}>
            <svg viewBox="0 0 16 16" aria-hidden="true"><path d="m4 4 8 8M12 4l-8 8" /></svg>
        </Button>
    </div>
</header>

<style>
.titlebar { display: flex; min-width: 0; height: 2.75rem; flex-shrink: 0; border-bottom: 1px solid var(--border-subtle); color: var(--text-secondary); }
.drag-region { display: flex; min-width: 0; flex: 1; align-items: center; padding-left: var(--space-5); cursor: default; }
.drag-region span { pointer-events: none; font-size: var(--settings-description-size); line-height: var(--settings-description-line-height); font-weight: var(--settings-heading-weight); }
.window-controls { display: flex; height: 100%; }
/* Adjacent caption buttons switch highlight immediately, without overlapping fade tails. */
.window-controls :global(button) { width: 2.75rem; min-height: 0; padding: 0; border: 0; border-radius: 0; background: transparent; color: var(--text-secondary); transition: none; }
.window-controls :global(button:hover:not(:disabled)) { background: var(--surface-hovered); color: var(--text-primary); }
.window-controls :global(.close-window:hover:not(:disabled)) { background: var(--surface-danger-hover); color: var(--text-danger); }
svg { width: 0.875rem; height: 0.875rem; fill: none; stroke: currentColor; stroke-width: 1.2; }
</style>
