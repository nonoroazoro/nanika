<script lang="ts">
import { integerError } from "./integerValue";
import { emptyValue } from "./values";
import type { ConfigurationSchema, ConfigurationValue } from "../types/Settings";

const { schema, value, label, id, onChange }: {
    schema: ConfigurationSchema;
    value: ConfigurationValue | undefined;
    label: string;
    id: string;
    onChange: (value: ConfigurationValue) => void;
} = $props();
let input = $state<HTMLInputElement>();
let previousLimit = $state<ConfigurationValue | undefined>();
const error = $derived(integerError(schema, value));
const numericText = $derived(typeof value === "number" || typeof value === "string" ? value : "");
// Native constraint validation is an external browser API, not derived state.
$effect(() =>
{
    input?.setCustomValidity(error ?? "");
});
</script>

<div class="integer-setting" class:invalid={error !== null} class:unlimited={value === null && schema.allowUnlimited}>
    <input
        bind:this={input}
        {id}
        type="text"
        inputmode="numeric"
        autocomplete="off"
        spellcheck={false}
        aria-label={label}
        aria-invalid={error !== null ? true : undefined}
        readonly={value === null && schema.allowUnlimited}
        tabindex={value === null && schema.allowUnlimited ? -1 : 0}
        required={value !== null || !schema.allowUnlimited}
        value={value === null && schema.allowUnlimited ? "Unlimited" : numericText}
        oninput={event =>
        {
            const raw = event.currentTarget.value;
            event.currentTarget.setCustomValidity(integerError(schema, raw) ?? "");
            onChange(raw);
        }}
    />
    {#if schema.allowUnlimited}
        <button
            type="button"
            class="limit-toggle"
            aria-label={`${label}: Unlimited`}
            aria-pressed={value === null}
            title="Unlimited"
            onclick={() =>
            {
                if (value === null)
                {
                    onChange(previousLimit ?? schema.default ?? emptyValue(schema));
                }
                else
                {
                    previousLimit = value;
                    onChange(null);
                }
            }}
        >
            <svg
                width="18"
                height="18"
                viewBox="0 0 24 24"
                fill="none"
                stroke="currentColor"
                stroke-width="1.7"
                stroke-linecap="round"
                aria-hidden="true"
            >
                <path d="M12 12c-2-3-3.2-4.5-5.5-4.5a4.5 4.5 0 1 0 0 9c2.3 0 3.5-1.5 5.5-4.5s3.2-4.5 5.5-4.5a4.5 4.5 0 1 1 0 9c-2.3 0-3.5-1.5-5.5-4.5Z" />
            </svg>
        </button>
    {/if}
</div>

<style>
.integer-setting { display: flex; align-items: center; width: 112px; height: 32px; border: 1px solid var(--border-subtle); border-radius: 6px; background: var(--surface-window); }
input { width: 100%; min-width: 0; padding: 5px 10px; border: 0; outline: 0; background: transparent; color: var(--text-primary); text-align: right; font-variant-numeric: tabular-nums; font-size: 12px; }
.unlimited input { color: var(--text-secondary); text-align: left; }
.limit-toggle { width: 32px; min-width: 32px; min-height: 26px; height: 26px; margin-right: 2px; padding: 4px; border-radius: 4px; color: var(--text-secondary); }
.limit-toggle[aria-pressed="true"] { color: var(--accent); background: var(--surface-selected); }
.limit-toggle:hover:not(:disabled) { background: var(--surface-hovered); color: var(--text-primary); }
.integer-setting.invalid { border-color: var(--border-danger); }
.integer-setting:focus-within { border-color: var(--border-window); }
input:focus-visible, .limit-toggle:focus-visible { outline: none; }
</style>
