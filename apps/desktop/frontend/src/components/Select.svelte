<script lang="ts">
import { Select } from "bits-ui";
import { onMount } from "svelte";
import { uiActivity } from "../ui/activity";

const { value, label, options, disabled = false, onChange }: {
    value: string;
    label: string;
    options: { value: string; label: string; disabled?: boolean; }[];
    disabled?: boolean;
    onChange: (value: string) => void;
} = $props();
let open = $state(false);
const selected = $derived(options.find(option => option.value === value));
onMount(() =>
    uiActivity.subscribe(activity =>
    {
        if (!activity.visible || !activity.focused)
        {
            open = false;
        }
    })
);
</script>

<Select.Root type="single" {value} {disabled} items={options} onValueChange={onChange} bind:open loop>
    <Select.Trigger class="control-button select-trigger" aria-label={label}>
        <span>{selected?.label ?? value}</span>
        <svg
            width="14"
            height="14"
            viewBox="0 0 24 24"
            fill="none"
            stroke="currentColor"
            stroke-width="1.6"
            aria-hidden="true"
        >
            <path d="m7 10 5 5 5-5" />
        </svg>
    </Select.Trigger>
    {#if $uiActivity.visible && $uiActivity.focused}
        <Select.Portal>
            <Select.Content
                class="popup-surface select-options"
                aria-label={label}
                sideOffset={5}
                collisionPadding={8}
                align="end"
                preventScroll={false}
                style="min-width: var(--bits-select-anchor-width); max-height: min(var(--bits-select-content-available-height), calc(100vh - 16px));"
            >
                {#each options as option (option.value)}
                    <Select.Item
                        class="popup-item"
                        value={option.value}
                        label={option.label}
                        disabled={option.disabled}
                    >
                        <span>{option.label}</span>
                        {#if option.value === value}
                            <svg
                                width="14"
                                height="14"
                                viewBox="0 0 24 24"
                                fill="none"
                                stroke="currentColor"
                                stroke-width="1.8"
                                aria-hidden="true"
                            >
                                <path d="m5 12 4 4L19 6" />
                            </svg>
                        {/if}
                    </Select.Item>
                {/each}
            </Select.Content>
        </Select.Portal>
    {/if}
</Select.Root>

<style>
:global(.select-trigger) { display: flex; justify-content: space-between; gap: var(--space-4); min-width: 7rem; font-weight: 400; }
:global(.select-trigger svg) { color: var(--text-secondary); flex-shrink: 0; }
:global(.select-options) { font-size: var(--font-control); }
:global(.select-options svg) { color: var(--accent); flex-shrink: 0; }
</style>
