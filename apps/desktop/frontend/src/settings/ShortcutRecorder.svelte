<script lang="ts">
import { onDestroy } from "svelte";
import { settingsBridge } from "../bridge/settingsBridge";
import { shortcutFromKey, shortcutKeys, shortcutModifiers } from "./shortcut";

const { value, registeredValue, onChange, onRecording }: {
    value: string;
    registeredValue: string;
    onChange: (value: string) => void;
    onRecording: (recording: boolean) => void;
} = $props();
let recording = $state(false);
let preview = $state<string[]>([]);
let error = $state<string | null>(null);
let active = true;
let pending = Promise.resolve();
let session = 0;
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
    const request = ++session;
    recording = enabled;
    preview = [];
    error = null;
    onRecording(enabled);
    pending = pending.then(async () =>
    {
        const accepted = await settingsBridge.recordShortcut(enabled && active && recording);
        if (active && request === session)
        {
            recording = accepted;
            onRecording(accepted);
        }
    }).catch(cause =>
    {
        if (active && request === session)
        {
            recording = false;
            onRecording(false);
            error = String(cause);
        }
        else
        {
            console.error(cause);
        }
    });
}

function cancel(): void
{
    if (recording)
    {
        setRecording(false);
    }
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
        setRecording(false);
    }
    else if (
        !event.repeat && !event.isComposing && !["Control", "Alt", "Shift", "Meta", "AltGraph"].includes(event.key)
    )
    {
        error = "Use Ctrl, Alt, or Command/Windows with a key.";
    }
}

onDestroy(() =>
{
    active = false;
    stopListening();
    cancel();
});
</script>

<svelte:window onblur={cancel} />
<div class="recorder">
    <button
        type="button"
        class:recording
        aria-label="Open launcher shortcut"
        aria-pressed={recording}
        aria-describedby={recording || error ? "shortcut-help" : undefined}
        onclick={() => setRecording(!recording)}
        onkeydown={capture}
        onkeyup={event =>
        {
            if (recording)
            {
                preview = shortcutModifiers(event);
            }
        }}
        onblur={cancel}
    >
        {#if recording && preview.length === 0}<span>Press shortcut…</span>
        {:else}{#each recording ? preview : shortcutKeys(value) as key, index (index)}
                {#if index > 0}<span class="separator" aria-hidden="true">+</span>{/if}<kbd>{key}</kbd>
            {/each}{/if}
    </button>
    {#if recording || error}
        <div class="help" id="shortcut-help" role={error ? "alert" : "status"}>
            {error ?? "Esc to cancel"}
        </div>
    {/if}
</div>

<style>
.recorder { position: relative; flex-shrink: 0; }
button { min-width: 156px; min-height: 32px; gap: 4px; padding: 4px 7px; border: 1px solid var(--border-subtle); border-radius: 6px; background: var(--surface-window); font-size: 12px; }
button.recording { border-color: var(--accent); }
kbd { min-width: 20px; padding: 2px 5px; border-radius: 3px; background: var(--surface-hovered); color: var(--text-secondary); font: inherit; line-height: 16px; }
.separator { color: var(--text-tertiary); font-size: 11px; }
.help { position: absolute; z-index: 1; top: calc(100% + 8px); right: 0; width: max-content; max-width: 260px; padding: 9px 12px; border: 1px solid var(--border-window); border-radius: 6px; background: var(--surface-form); color: var(--text-secondary); font-size: 12px; line-height: 1.5; box-shadow: 0 4px 12px rgb(0 0 0 / 8%); }
.help[role="alert"] { color: var(--text-danger); }
button:focus-visible { background: var(--surface-hovered); }
</style>
