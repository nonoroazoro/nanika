<script lang="ts">
import InfinityIcon from "../components/icons/InfinityIcon.svelte";
import Button from "../components/ui/Button.svelte";
import Input from "../components/ui/Input.svelte";
import { integerError } from "./integerValue";
import { emptyValue } from "./values";
import type { ConfigurationProperty } from "../generated/ConfigurationProperty";
import type { ConfigurationSchema } from "../generated/ConfigurationSchema";
import type { JsonValue } from "../generated/serde_json/JsonValue";

const { schema, value, label, id, onChange, onCommit }: {
    schema: ConfigurationSchema | ConfigurationProperty;
    value: JsonValue | undefined;
    label: string;
    id: string;
    onChange: (value: JsonValue) => void;
    onCommit: () => void;
} = $props();
let input = $state<HTMLInputElement>();
let previousLimit = $state<JsonValue | undefined>();
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
                    onChange(previousLimit ?? ("default" in schema ? schema.default : undefined) ?? emptyValue(schema));
                }
                else
                {
                    previousLimit = value;
                    onChange(null);
                }
                onCommit();
            }}
        >
            <InfinityIcon size={18} strokeWidth={1.7} />
        </Button>
    {/if}
</div>

<style>
.integer-setting { display: flex; align-items: center; width: var(--settings-control-width); padding: 0; }
.integer-setting :global(input) { text-align: right; font-variant-numeric: tabular-nums; }
.unlimited :global(input) { color: var(--text-secondary); text-align: left; }
.integer-setting :global(.limit-toggle) { width: 28px; min-width: 28px; min-height: calc(var(--control-height) - 6px); height: calc(var(--control-height) - 6px); margin-right: 2px; padding: 4px; border-radius: 4px; color: var(--text-secondary); }
.integer-setting :global(.limit-toggle[aria-pressed="true"]) { color: var(--accent); background: var(--surface-selected); }
.integer-setting :global(.limit-toggle:hover:not(:disabled)) { background: var(--surface-hovered); color: var(--text-primary); }
.integer-setting.invalid { border-color: var(--border-danger); }
</style>
