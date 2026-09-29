<script lang="ts">
import { Tooltip } from "bits-ui";
import { onMount } from "svelte";
import Button from "./Button.svelte";
import type { TruncatedButtonProps } from "../../types/TruncatedButtonProps";
import { uiActivity } from "../../ui/activity";

let { label, icon, ref = $bindable(), variant, disabled = false, ...attributes }: TruncatedButtonProps = $props();
let text = $state<HTMLSpanElement>();
let open = $state(false);
const active = $derived($uiActivity.visible && $uiActivity.focused);
const triggerProps: Tooltip.TriggerProps = $derived({ ...attributes, id: attributes.id ?? undefined, disabled });

onMount(() =>
    uiActivity.subscribe(activity =>
    {
        if (!activity.visible || !activity.focused)
        {
            open = false;
        }
    })
);

function _setOpen(next: boolean): void
{
    // Measure only on interaction, after layout; fitting labels need no duplicate hint.
    open = next && active && Boolean(text && text.scrollWidth > text.clientWidth);
}
</script>

<Tooltip.Root bind:open={() => open && active, _setOpen} disabled={!active}>
    <Tooltip.Trigger {...triggerProps}>
        {#snippet child({ props })}
            <Button {...props} {variant} bind:ref>
                {@render icon?.()}
                <span class="truncated-label" bind:this={text}>{label}</span>
            </Button>
        {/snippet}
    </Tooltip.Trigger>
    {#if active}
        <Tooltip.Portal>
            <Tooltip.Content class="truncated-label-tooltip" side="right" sideOffset={4} collisionPadding={8}>
                {label}
            </Tooltip.Content>
        </Tooltip.Portal>
    {/if}
</Tooltip.Root>

<style>
.truncated-label { display: block; min-width: 0; overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
:global(.truncated-label-tooltip) { z-index: 10; max-width: min(20rem, calc(100vw - 16px)); padding: var(--space-2); border: 1px solid var(--border-subtle); border-radius: var(--control-radius); background: var(--surface-form); color: var(--text-primary); box-shadow: var(--shadow-popup); font-size: var(--font-control); line-height: var(--control-line-height); overflow-wrap: anywhere; }
</style>
