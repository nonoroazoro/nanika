<script lang="ts">
import Button from "../components/Button.svelte";
import Input from "../components/Input.svelte";
import { integerError } from "./integerValue";
import { emptyValue } from "./values";
import type { ConfigurationSchema, ConfigurationValue } from "../types/Settings";

const { schema, value, label, id, onChange, onCommit }: {
    schema: ConfigurationSchema;
    value: ConfigurationValue | undefined;
    label: string;
    id: string;
    onChange: (value: ConfigurationValue) => void;
    onCommit: () => void;
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

<div
    class="control-field integer-setting"
    class:invalid={error !== null}
    class:unlimited={value === null && schema.allowUnlimited}
>
    <Input
        variant="embedded"
        bind:ref={input}
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
        value={value === null && schema.allowUnlimited ? "Unlimited" : String(numericText)}
        oninput={event =>
        {
            const raw = event.currentTarget.value;
            event.currentTarget.setCustomValidity(integerError(schema, raw) ?? "");
            onChange(raw);
        }}
    />
    {#if schema.allowUnlimited}
        <Button
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
                onCommit();
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
        </Button>
    {/if}
</div>

<style>
.integer-setting { display: flex; align-items: center; width: 7rem; padding: 0; }
.integer-setting :global(input) { text-align: right; font-variant-numeric: tabular-nums; }
.unlimited :global(input) { color: var(--text-secondary); text-align: left; }
.integer-setting :global(.limit-toggle) { width: 32px; min-width: 32px; min-height: 26px; height: 26px; margin-right: 2px; padding: 4px; border-radius: 4px; color: var(--text-secondary); }
.integer-setting :global(.limit-toggle[aria-pressed="true"]) { color: var(--accent); background: var(--surface-selected); }
.integer-setting :global(.limit-toggle:hover:not(:disabled)) { background: var(--surface-hovered); color: var(--text-primary); }
.integer-setting.invalid { border-color: var(--border-danger); }
</style>
