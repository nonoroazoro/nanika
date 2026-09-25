<script lang="ts">
import Button from "../components/Button.svelte";
import Input from "../components/Input.svelte";
import SettingsValue from "./SettingsValue.svelte";
import IntegerSetting from "./IntegerSetting.svelte";
import Switch from "../components/Switch.svelte";
import { emptyValue, fieldTitle } from "./values";
import { stringError } from "./stringValue";
import type { ConfigurationSchema, ConfigurationValue } from "../types/Settings";

const { schema, value, label, id, onChange, onCommit }: {
    schema: ConfigurationSchema;
    value: ConfigurationValue | undefined;
    label: string;
    id: string;
    onChange: (value: ConfigurationValue) => void;
    onCommit: () => void;
} = $props();
const array = $derived(Array.isArray(value) ? value : []);
const object = $derived(value && typeof value === "object" && !Array.isArray(value) ? value : {});
let visibleCount = $state(20);
let stringInput = $state<HTMLInputElement>();
const currentStringError = $derived(schema.type === "string" ? stringError(schema, value) : null);
$effect(() => stringInput?.setCustomValidity(currentStringError ?? ""));
function _change(next: ConfigurationValue): void
{
    onChange(next);
    onCommit();
}
</script>

{#if schema.type === "boolean"}
    <Switch {id} {label} checked={value === true} onChange={_change} />
{:else if schema.type === "integer"}
    <IntegerSetting {id} {schema} {value} {label} {onChange} {onCommit} />
{:else if schema.type === "string"}
    <Input
        bind:ref={stringInput}
        {id}
        type="text"
        aria-label={label}
        spellcheck={schema.format !== "path"}
        maxlength={schema.maxLength ?? 4096}
        value={typeof value === "string" ? value : ""}
        oninput={event => onChange(event.currentTarget.value)}
        aria-invalid={currentStringError !== null ? true : undefined}
    />
{:else if schema.type === "array" && schema.items}
    <div class="array" role="group" aria-label={label}>
        {#each array.slice(0, visibleCount) as item, index (index)}
            <div class="array-item" class:structured={schema.items.type === "object"}>
                <div class="item-value">
                    <SettingsValue
                        schema={schema.items}
                        value={item}
                        label={`${label}, entry ${index + 1}`}
                        id={`${id}-${index}`}
                        {onCommit}
                        onChange={next => onChange(array.map((current, position) => position === index ? next : current))}
                    />
                </div>
                <Button
                    class="remove"
                    aria-label={`Remove ${label} entry ${index + 1}`}
                    onclick={() => _change(array.filter((_, position) => position !== index))}
                >
                    Remove
                </Button>
            </div>
        {/each}
        <div class="array-actions">
            <Button
                disabled={array.length >= (schema.maxItems ?? 0)}
                onclick={() =>
                {
                    if (schema.items)
                    {
                        _change([...array, emptyValue(schema.items)]);
                        visibleCount = array.length + 1;
                    }
                }}
            >
                Add entry
            </Button>
            {#if array.length > visibleCount}<Button
                    onclick={() =>
                    {
                        visibleCount += 20;
                    }}
                >
                    Show more ({array.length - visibleCount})
                </Button>{/if}
            <span>{array.length} / {schema.maxItems}</span>
        </div>
    </div>
{:else if schema.type === "object"}
    <div class="object" role="group" aria-label={label}>
        {#each Object.entries(schema.properties) as [key, child] (key)}
            <div class="object-field">
                <div class="field-heading">
                    <span>{fieldTitle(key)}</span>
                    {#if !schema.required.includes(key)}
                        <div class="optional">
                            <span>Include</span><Switch
                                checked={Object.hasOwn(object, key)}
                                label={`Include ${fieldTitle(key)}`}
                                onChange={checked =>
                                {
                                    const next = { ...object };
                                    if (checked)
                                    {
                                        next[key] = emptyValue(child);
                                    }
                                    else
                                    {
                                        delete next[key];
                                    }
                                    _change(next);
                                }}
                            />
                        </div>
                    {/if}
                </div>
                {#if Object.hasOwn(object, key) || schema.required.includes(key)}
                    <SettingsValue
                        schema={child}
                        value={object[key]}
                        label={`${label}, ${fieldTitle(key)}`}
                        id={`${id}-${key}`}
                        {onCommit}
                        onChange={next => onChange({ ...object, [key]: next })}
                    />
                {/if}
            </div>
        {/each}
    </div>
{/if}

<style>
.optional { display: flex; align-items: center; gap: var(--space-2); }
.array, .object { display: grid; gap: var(--space-3); }
.array-item { display: flex; align-items: flex-start; gap: var(--space-2); }
.array-item.structured { padding: var(--space-4); border: 1px solid var(--border-subtle); border-radius: var(--radius-row); }
.item-value { flex: 1; min-width: 0; }
.array-item :global(.remove) { flex: 0 0 auto; }
.array-actions, .field-heading { display: flex; align-items: center; justify-content: space-between; gap: var(--space-2); }
.array-actions { justify-content: flex-start; }
.array-actions span, .optional { color: var(--text-secondary); font-size: var(--font-meta); }
.object-field { display: grid; gap: var(--space-2); }
.field-heading { font-size: var(--font-meta); }
</style>
