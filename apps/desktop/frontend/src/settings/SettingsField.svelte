<script lang="ts">
import type { Snippet } from "svelte";
import { uiActivity } from "../ui/activity";
import type { OperationProgress } from "../types/OperationProgress";

const { busy = false, label, startedAt, progress, onCommit, children }: {
    busy?: boolean;
    label: string;
    startedAt?: number;
    progress?: OperationProgress;
    onCommit?: () => void;
    children: Snippet;
} = $props();
let delayed = $state(false);
let fieldset: HTMLFieldSetElement;
let pointerInside = false;
let pendingFocusCommit = false;

function _focusOut(event: FocusEvent): void
{
    if (event.relatedTarget instanceof Node && fieldset.contains(event.relatedTarget))
    {
        return;
    }
    // A pointer action inside a compound field owns the final value. On macOS
    // buttons may not receive focus, so relatedTarget alone cannot identify it.
    if (pointerInside)
    {
        pendingFocusCommit = true;
        return;
    }
    onCommit?.();
}

function _finishPointer(): void
{
    pointerInside = false;
    if (pendingFocusCommit)
    {
        pendingFocusCommit = false;
        if (!fieldset.contains(document.activeElement))
        {
            onCommit?.();
        }
    }
}

const description = $derived(
    progress
        ? `${progress.label}${progress.total === null ? "" : `: ${progress.completed} of ${progress.total}`}`
        : `Updating ${label}`
);

// Synchronize a one-shot presentation delay. It never delays operation completion.
$effect(() =>
{
    delayed = false;
    if (!busy || !$uiActivity.visible || startedAt === undefined)
    {
        return;
    }
    const timer = setTimeout(() =>
    {
        delayed = true;
    }, Math.max(0, 1000 - (Date.now() - startedAt)));
    return () =>
    {
        clearTimeout(timer);
    };
});
</script>

<svelte:window
    onclick={_finishPointer}
    onpointercancel={_finishPointer}
    onblur={_finishPointer}
    onpointerup={() =>
    {
        pointerInside = false;
    }}
/>
<div class="setting-field">
    <fieldset
        bind:this={fieldset}
        disabled={busy}
        aria-busy={busy || undefined}
        onfocusout={_focusOut}
        onpointerdowncapture={() =>
        {
            pointerInside = true;
        }}
        onchange={event =>
        {
            // Native Enter can commit without leaving the field.
            if (event.target === document.activeElement)
            {
                onCommit?.();
            }
        }}
    >
        {@render children()}
    </fieldset>
    {#if delayed && busy}
        <div
            class="progress"
            class:determinate={progress?.total != null}
            class:visible={$uiActivity.visible}
            role="progressbar"
            aria-label={description}
            title={description}
            aria-valuemin={0}
            aria-valuemax={progress?.total ?? undefined}
            aria-valuenow={progress?.total != null ? progress.completed : undefined}
        >
            {#if progress?.total != null}
                <span class="fill" style:width={`${progress.completed / progress.total * 100}%`}></span>
            {:else}
                <span class="indeterminate"></span>
            {/if}
        </div>
    {/if}
</div>

<style>
.setting-field { position: relative; min-width: 0; }
fieldset { min-width: 0; margin: 0; border: 0; padding: 0; }
fieldset[aria-busy="true"] :global(button:disabled:not([disabled])) { opacity: 1; cursor: pointer; }
.progress { position: absolute; inset: auto 0 -7px; width: 100%; height: 2px; color: var(--accent); border-radius: 1px; overflow: hidden; background: var(--border-subtle); }
.fill { display: block; height: 100%; background: currentColor; }
.indeterminate { display: block; width: 35%; height: 100%; border-radius: inherit; background: currentColor; }
.progress.visible .indeterminate { animation: progress-sweep var(--motion-progress-duration) ease-in-out infinite alternate; }
@keyframes progress-sweep { from { transform: translateX(0); } to { transform: translateX(185%); } }
@media (prefers-reduced-motion: reduce) { .progress.visible .indeterminate { animation: none; } }
</style>
