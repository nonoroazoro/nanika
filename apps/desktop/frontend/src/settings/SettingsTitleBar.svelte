<script lang="ts">
import Button from "../components/Button.svelte";
import { uiActivity } from "../ui/activity";
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
    <div class="window-controls" data-visible={$uiActivity.visible} role="group" aria-label="Window controls">
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
    <div class="titlebar-separator" aria-hidden="true"></div>
</header>

<style>
/* Controls and separator occupy disjoint grid rows, never overlapping layers. */
.titlebar { display: grid; grid-template-columns: minmax(0, 1fr) 138px; grid-template-rows: minmax(0, 1fr) 1px; min-width: 0; height: 2.75rem; flex-shrink: 0; color: var(--text-secondary); }
.titlebar-separator { grid-column: 1 / -1; background: var(--border-subtle); }
.drag-region { display: flex; min-width: 0; align-items: center; padding-left: var(--space-5); cursor: default; }
.drag-region span { pointer-events: none; font-size: var(--settings-description-size); line-height: var(--settings-description-line-height); font-weight: var(--settings-heading-weight); }
/* Adjacent visual and pointer regions share the same fixed grid geometry. */
.window-controls { --caption-feedback-duration: 150ms; display: grid; grid-template-columns: repeat(3, 46px); min-height: 0; }
.window-controls :global(button) { --caption-hover-background: var(--settings-caption-hover); position: relative; isolation: isolate; display: flex; align-items: center; justify-content: center; width: auto; min-width: 0; height: auto; min-height: 0; margin: 0; padding: 0; border: 0; border-radius: 0; background: transparent; color: inherit; cursor: default; transition: color var(--caption-feedback-duration) ease-out; }
/* Keep the fill constant and fade a non-interactive layer. Background-color
   transitions showed a stale painted frame at completion in Windows WebView2. */
.window-controls :global(button)::before { content: ""; position: absolute; inset: 0; z-index: -1; pointer-events: none; background: var(--caption-hover-background); opacity: 0; transition: opacity var(--caption-feedback-duration) ease-out; }
.window-controls :global(button:hover:not(:disabled, [aria-disabled="true"])) { background: transparent; color: inherit; }
.window-controls :global(button:hover:not(:disabled, [aria-disabled="true"]))::before { opacity: 1; }
.window-controls :global(.close-window) { --caption-hover-background: var(--settings-caption-close-hover); }
.window-controls :global(.close-window:hover:not(:disabled, [aria-disabled="true"])) { color: var(--settings-caption-close-foreground); }
/* Visibility settles motion without tying hover feedback to focus changes. */
.window-controls[data-visible="false"] { --caption-feedback-duration: 0ms; }
@media (prefers-reduced-motion: reduce) { .window-controls { --caption-feedback-duration: 0ms; } }
svg { pointer-events: none; width: 16px; height: 16px; fill: none; stroke: currentColor; stroke-width: 1.2; }
</style>
