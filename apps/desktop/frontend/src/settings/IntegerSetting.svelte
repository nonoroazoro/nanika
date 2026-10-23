<script lang="ts">
import { localization } from "../localization/state.svelte";
import { validationErrorText } from "./validation-presentation";
import Input from "../components/ui/Input.svelte";
import { integerError } from "./integerValue";
import type { ConfigurationSchema } from "../generated/ConfigurationSchema";
import type { JsonValue } from "../generated/serde_json/JsonValue";

const { schema, value, label, id, onChange }: {
    schema: ConfigurationSchema;
    value: JsonValue | undefined;
    label: string;
    id: string;
    onChange: (value: JsonValue) => void;
} = $props();
let input = $state<HTMLInputElement>();
const error = $derived(integerError(schema, value));
$effect(() => input?.setCustomValidity(validationErrorText(error, message => localization.text(message))));
</script>

<Input
    bind:ref={input}
    {id}
    class="integer-setting"
    type="text"
    inputmode="numeric"
    autocomplete="off"
    spellcheck={false}
    aria-label={label}
    aria-invalid={error !== null ? true : undefined}
    required
    value={typeof value === "number" || typeof value === "string" ? String(value) : ""}
    oninput={event =>
    {
        const raw = event.currentTarget.value;
        event.currentTarget.setCustomValidity(
            validationErrorText(integerError(schema, raw), message => localization.text(message))
        );
        onChange(raw);
    }}
/>

<style>
:global(.integer-setting) { width: var(--settings-control-width); text-align: right; font-variant-numeric: tabular-nums; }
:global(.integer-setting[aria-invalid="true"]) { border-color: var(--border-danger); }
</style>
