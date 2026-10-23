<script lang="ts">
import { localization } from "../localization/state.svelte";
import type { LocalizedMessage } from "../generated/LocalizedMessage";
import type { OperationError } from "../generated/OperationError";
import { operationErrorText, operationFailure } from "../localization/operation-errors";

import { isCompositionKey } from "../ui/keyboard";
import Button from "../components/ui/Button.svelte";
import { onDestroy } from "svelte";
import { settingsBridge } from "../bridge/settingsBridge";
import ShortcutKeys from "../components/ui/ShortcutKeys.svelte";
import { shortcutFromKey, shortcutKeys, shortcutModifiers } from "./shortcut";

const { value, registeredValue, onChange }: {
    value: string;
    registeredValue: string;
    onChange: (value: string) => void;
} = $props();
let recording = $state(false);
let preview = $state<string[]>([]);
let confirmed = $state(false);
let error = $state<LocalizedMessage | OperationError | null>(null);
const errorText = $derived.by(() =>
{
    if (!error)
    {
        return null;
    }
    return "kind" in error
        ? operationErrorText(error, localization.locale, message => localization.text(message))
        : localization.text(error);
});
let recorderButton = $state<HTMLButtonElement>();
let active = true;
let pending = Promise.resolve();
let session = 0;
let confirmationTimer: ReturnType<typeof setTimeout> | undefined;
const stopListening = settingsBridge.listenShortcut(() =>
{
    if (recording)
    {
        onChange(registeredValue);
        setRecording(false);
    }
});

// Serialize start/stop requests so a late start cannot outlive blur or unmount.
function setRecording(enabled: boolean): void
{
    if (enabled)
    {
        clearConfirmation();
    }
    const request = ++session;
    recording = enabled;
    preview = [];
    error = null;
    pending = pending.then(async () =>
    {
        const accepted = await settingsBridge.recordShortcut(enabled && active && recording);
        if (active && request === session)
        {
            recording = accepted;
        }
    }).catch(cause =>
    {
        if (active && request === session)
        {
            console.error("Shortcut recording could not be updated", cause);
            recording = false;
            error = operationFailure(cause, "shortcutRecordingFailed");
        }
        else
        {
            console.error(cause);
        }
    });
}

function clearConfirmation(): void
{
    if (confirmationTimer !== undefined)
    {
        clearTimeout(confirmationTimer);
        confirmationTimer = undefined;
    }
    confirmed = false;
}

function confirm(): void
{
    clearConfirmation();
    confirmed = true;
    confirmationTimer = setTimeout(() =>
    {
        confirmationTimer = undefined;
        if (active)
        {
            confirmed = false;
        }
    }, 900);
}

function cancel(): void
{
    if (recording)
    {
        setRecording(false);
    }
}

function toggleRecording(): void
{
    recorderButton?.focus({ preventScroll: true });
    setRecording(!recording);
}

// Capture on the focused recorder before window-level commands can handle the key.
function capture(event: KeyboardEvent): void
{
    if (!recording)
    {
        return;
    }
    const modifiers = shortcutModifiers(event);
    if (modifiers.length === 0 && (event.key === "Escape" || event.key === "Tab"))
    {
        if (event.key === "Escape")
        {
            event.preventDefault();
        }
        cancel();
        return;
    }
    event.preventDefault();
    event.stopPropagation();
    preview = modifiers;
    const shortcut = shortcutFromKey(event);
    if (shortcut)
    {
        onChange(shortcut);
        confirm();
        setRecording(false);
    }
    else if (
        !event.repeat && !isCompositionKey(event)
        && !["Control", "Alt", "Shift", "Meta", "AltGraph"].includes(event.key)
    )
    {
        error = { message: "settings-shortcut-modifiers" };
    }
}

function updatePreview(event: KeyboardEvent): void
{
    if (recording)
    {
        preview = shortcutModifiers(event);
    }
}

onDestroy(() =>
{
    active = false;
    clearConfirmation();
    stopListening();
    cancel();
});
</script>

<svelte:window onblur={cancel} />
<div class="recorder">
    <Button
        variant="outline"
        bind:ref={recorderButton}
        class={{ recording }}
        aria-label={localization.text({ message: "settings-shortcut-label" })}
        aria-pressed={recording}
        aria-describedby={error ? "shortcut-help" : undefined}
        onclick={toggleRecording}
        onblur={cancel}
        onkeydown={capture}
        onkeyup={updatePreview}
    >
        {#if recording && preview.length === 0}<span class="recording-prompt">{
                localization.text({ message: "settings-shortcut-recording" })
            }</span>
        {:else}<ShortcutKeys
                keys={recording ? preview : shortcutKeys(value)}
                success={confirmed}
            />{/if}
    </Button>
    {#if error}
        <div class="help" id="shortcut-help" role="alert">{errorText}</div>
    {/if}
</div>

<style>
.recorder { position: relative; flex-shrink: 0; }
.recording-prompt { color: var(--text-secondary); }
.recorder :global(button) { min-width: 160px; gap: var(--space-1); }
.help { position: absolute; z-index: 1; top: calc(100% + 8px); right: 0; width: max-content; max-width: 260px; padding: var(--space-2) var(--space-3); border: 1px solid var(--border-window); border-radius: var(--control-radius); background: var(--surface-form); color: var(--text-secondary); font-size: var(--settings-description-size); line-height: var(--settings-description-line-height); box-shadow: var(--shadow-popup); }
.help[role="alert"] { color: var(--text-danger); overflow-wrap: anywhere; white-space: pre-wrap; max-height: 30vh; overflow: auto; }
.recorder :global(button.recording), .recorder :global(button.recording:hover), .recorder :global(button.recording:focus-visible) { border-color: var(--accent); background: var(--surface-accent); }
</style>
