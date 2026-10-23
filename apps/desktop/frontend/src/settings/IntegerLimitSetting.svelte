<script lang="ts">
import { localization } from "../localization/state.svelte";
import { validationErrorText } from "./validation-presentation";
import { tick } from "svelte";
import InfinityIcon from "../components/icons/InfinityIcon.svelte";
import Button from "../components/ui/Button.svelte";
import Input from "../components/ui/Input.svelte";
import { integerError, integerLimitDraft } from "./integerValue";
import type { ConfigurationSchema } from "../generated/ConfigurationSchema";
import type { JsonValue } from "../generated/serde_json/JsonValue";

const { schema, value, label, id, onChange, onCommit }: {
    schema: ConfigurationSchema;
    value: JsonValue | undefined;
    label: string;
    id: string;
    onChange: (value: JsonValue) => void;
    onCommit: () => void | Promise<void>;
} = $props();
let input = $state<HTMLInputElement>();
let control = $state<HTMLDivElement>();
const limit = $derived(integerLimitDraft(value));
const unlimited = $derived(limit?.unlimited ?? false);
const error = $derived(integerError(schema, limit?.value));
$effect(() => input?.setCustomValidity(validationErrorText(error, message => localization.text(message))));

async function _finite(): Promise<void>
{
    if (!limit)
    {
        return;
    }
    onChange({ ...limit, unlimited: false });
    const owner = control;
    await onCommit();
    await tick();
    // Persistence temporarily disables the field. Restore editing only after it
    // settles, and never steal focus after navigation or another user action.
    if (
        owner?.isConnected && !unlimited && document.hasFocus()
        && (document.activeElement === document.body || owner.contains(document.activeElement))
    )
    {
        input?.focus({ preventScroll: true });
        input?.select();
    }
}

function _unlimited(): void
{
    if (!limit || unlimited)
    {
        return;
    }
    // Never persist an invalid remembered value or silently replace the user's draft.
    if (error)
    {
        input?.reportValidity();
        input?.focus();
        return;
    }
    onChange({ ...limit, unlimited: true });
    onCommit();
}
</script>

<div
    bind:this={control}
    class="integer-limit control-field"
    class:unlimited
    class:invalid={error !== null}
    role="group"
    aria-label={label}
>
    <span class="selection" aria-hidden="true"></span>
    <div class="finite">
        <!-- Keep one text renderer in both modes so native input metrics never
             change when the finite segment becomes a selection target. -->
        <Input
            variant="embedded"
            bind:ref={input}
            {id}
            type="text"
            inputmode="numeric"
            autocomplete="off"
            spellcheck={false}
            aria-label={label}
            aria-hidden={unlimited ? true : undefined}
            aria-invalid={error !== null ? true : undefined}
            readonly={unlimited}
            tabindex={unlimited ? -1 : 0}
            required
            value={String(limit?.value ?? "")}
            oninput={event =>
            {
                const raw = event.currentTarget.value;
                event.currentTarget.setCustomValidity(
                    validationErrorText(integerError(schema, raw), message => localization.text(message))
                );
                onChange({ unlimited: false, value: raw });
            }}
        />
        {#if unlimited}
            <Button
                class="finite-toggle"
                aria-label={localization.text({ message: "settings-limit-finite-label", args: { label, value: limit?.value ?? "" } })}
                title={localization.text({ message: "settings-limit-finite-title" })}
                onclick={_finite}
            />
        {/if}
    </div>
    <Button
        class="infinity-toggle"
        aria-label={localization.text({ message: "settings-limit-unlimited-label", args: { label } })}
        aria-pressed={unlimited}
        title={localization.text({ message: "settings-limit-unlimited-title" })}
        onclick={_unlimited}
    ><InfinityIcon size={18} strokeWidth={1.7} /></Button>
</div>

<style>
.integer-limit { --limit-number-width: calc(3ch + 1.25rem); --limit-infinity-width: var(--limit-number-width); position: relative; isolation: isolate; display: inline-grid; grid-template-columns: var(--limit-number-width) var(--limit-infinity-width); width: auto; padding: 2px; gap: 2px; font-variant-numeric: tabular-nums; background: var(--surface-selected); }
.selection { position: absolute; z-index: -1; top: 2px; bottom: 2px; left: 2px; width: var(--limit-number-width); border-radius: calc(var(--control-radius) - 2px); background: var(--control-surface); box-shadow: 0 1px 3px rgb(0 0 0 / 10%); transform: translateX(0); transition: transform var(--motion-switch) var(--motion-ease); pointer-events: none; }
.unlimited .selection { transform: translateX(calc(var(--limit-number-width) + 2px)); }
.finite { position: relative; display: flex; min-width: 0; align-items: center; }
.finite :global(input), .integer-limit :global(button) { min-height: calc(var(--control-height) - 6px); height: calc(var(--control-height) - 6px); padding: 0 4px; border: 0; border-radius: calc(var(--control-radius) - 2px); text-align: center; font-variant-numeric: tabular-nums; }
.finite :global(input) { padding: 0 6px; }
.unlimited .finite :global(input) { color: var(--text-tertiary); pointer-events: none; }
.integer-limit :global(.finite-toggle) { position: absolute; inset: 0; width: 100%; }
.integer-limit :global(.infinity-toggle) { width: var(--limit-infinity-width); color: var(--text-secondary); }
.integer-limit :global(.infinity-toggle[aria-pressed="true"]) { color: var(--accent); }
.integer-limit :global(button:hover:not(:disabled)) { background: var(--surface-hovered); }
.integer-limit.invalid { border-color: var(--border-danger); }
:global(:root[data-ui-active="false"]) .selection { transition: none; }
@media (prefers-reduced-motion: reduce) { .selection { transition: none; } }
</style>
