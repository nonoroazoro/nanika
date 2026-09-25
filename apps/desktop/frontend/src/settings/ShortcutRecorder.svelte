<script lang="ts">
import Button from "../components/Button.svelte";
import { onDestroy } from "svelte";
import { settingsBridge } from "../bridge/settingsBridge";
import ShortcutKeys from "../components/ShortcutKeys.svelte";
import { shortcutFromKey, shortcutKeys, shortcutModifiers } from "./shortcut";

const { value, registeredValue, onChange }: {
    value: string;
    registeredValue: string;
    onChange: (value: string) => void;
} = $props();
let recording = $state(false);
let preview = $state<string[]>([]);
let confirmed = $state(false);
let error = $state<string | null>(null);
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
            error = "Shortcut recording is unavailable. Try again.";
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
        !event.repeat && !event.isComposing && !["Control", "Alt", "Shift", "Meta", "AltGraph"].includes(event.key)
    )
    {
        error = "Use Ctrl, Alt, or Command/Windows with a key.";
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

<svelte:window onblur={cancel} onkeydown={capture} onkeyup={updatePreview} />
<div class="recorder">
    <Button
        variant="outline"
        bind:ref={recorderButton}
        class={{ recording }}
        aria-label="Open launcher shortcut"
        aria-pressed={recording}
        aria-describedby={error ? "shortcut-help" : undefined}
        onclick={toggleRecording}
        onblur={cancel}
    >
        {#if recording && preview.length === 0}<span>Press shortcut…</span>
        {:else}<ShortcutKeys
                keys={recording ? preview : shortcutKeys(value)}
                success={confirmed}
            />{/if}
    </Button>
    {#if error}
        <div class="help" id="shortcut-help" role="alert">{error}</div>
    {/if}
</div>

<style>
.recorder { position: relative; flex-shrink: 0; }
.recorder :global(button) { min-width: 156px; gap: var(--space-1); }
.help { position: absolute; z-index: 1; top: calc(100% + 8px); right: 0; width: max-content; max-width: 260px; padding: 9px 12px; border: 1px solid var(--border-window); border-radius: var(--control-radius); background: var(--surface-form); color: var(--text-secondary); font-size: var(--font-control); line-height: 1.5; box-shadow: 0 4px 12px rgb(0 0 0 / 8%); }
.help[role="alert"] { color: var(--text-danger); }
.recorder :global(button.recording), .recorder :global(button.recording:hover), .recorder :global(button.recording:focus-visible) { border-color: var(--accent); background: var(--surface-accent); }
</style>
