<script lang="ts">
import { onDestroy, onMount } from "svelte";
import { settingsBridge } from "../bridge/settingsBridge";
import Switch from "../components/Switch.svelte";
import Select from "../components/Select.svelte";
import ShortcutRecorder from "./ShortcutRecorder.svelte";
import SettingsActions from "./SettingsActions.svelte";
import type { HostPreferences, StartupStatus } from "../types/Settings";

const { preferences, draft, onChange, onSaved }: {
    preferences: HostPreferences;
    draft: HostPreferences;
    onChange: (value: HostPreferences) => void;
    onSaved: (value: HostPreferences) => void;
} = $props();
let saving = $state(false);
let recording = $state(false);
let error = $state<string | null>(null);
let startup = $state<StartupStatus | null>(null);
let startupBusy = $state(false);
let startupError = $state<string | null>(null);
let active = true;
const dirty = $derived(JSON.stringify(preferences) !== JSON.stringify(draft));
const startupStatusMessage = $derived.by(() =>
{
    if (startup === "requiresApproval")
    {
        return "Turn this on to open System Settings and allow Nanika.";
    }
    if (startup === "needsRepair")
    {
        return "Turn this on to repair launch at login.";
    }
    return null;
});

onMount(() =>
{
    void settingsBridge.readStartup().then(value =>
    {
        if (active)
        {
            startup = value;
        }
    })
        .catch(cause =>
        {
            if (active)
            {
                console.error("Startup settings could not be loaded", cause);
            }
        });
});
onDestroy(() =>
{
    active = false;
});

async function save(event: SubmitEvent): Promise<void>
{
    event.preventDefault();
    if (saving || !dirty)
    {
        return;
    }
    saving = true;
    error = null;
    try
    {
        const saved = await settingsBridge.saveHost(structuredClone(draft));
        if (active)
        {
            onSaved(saved);
        }
    }
    catch (cause)
    {
        if (active)
        {
            console.error("General settings could not be saved", cause);
            error = "Changes could not be saved. Try again.";
        }
    }
    finally
    {
        if (active)
        {
            saving = false;
        }
    }
}

async function changeStartup(enabled: boolean): Promise<void>
{
    if (startupBusy)
    {
        return;
    }
    startupBusy = true;
    startupError = null;
    try
    {
        const value = await settingsBridge.setStartup(enabled);
        if (active)
        {
            startup = value;
        }
    }
    catch (cause)
    {
        if (active)
        {
            console.error("Startup settings could not be updated", cause);
            startupError = "Launch at login could not be updated. Try again.";
        }
    }
    finally
    {
        if (active)
        {
            startupBusy = false;
        }
    }
}
</script>

<div class="general-page">
    <h1>General</h1>
    <form onsubmit={save}>
        <fieldset disabled={saving}>
            <section aria-labelledby="launcher-settings">
                <h2 id="launcher-settings">Launcher</h2>
                <div class="group">
                    <div class="row">
                        <span id="shortcut-label">Open launcher</span>
                        <ShortcutRecorder
                            value={draft.launcherShortcut}
                            registeredValue={preferences.launcherShortcut}
                            onChange={launcherShortcut => onChange({ ...draft, launcherShortcut })}
                            onRecording={value =>
                            {
                                recording = value;
                            }}
                        />
                    </div>
                    <div class="row">
                        <span>Hide when focus is lost</span>
                        <Switch
                            label="Hide when focus is lost"
                            checked={draft.hideOnBlur}
                            onChange={checked => onChange({ ...draft, hideOnBlur: checked })}
                        />
                    </div>
                </div>
            </section>
            <section aria-labelledby="appearance-settings">
                <h2 id="appearance-settings">Appearance</h2>
                <div class="group">
                    <div class="row">
                        <span>Theme</span>
                        <Select
                            label="Theme"
                            value={draft.theme}
                            options={[{ value: "system", label: "System" }, { value: "light", label: "Light" }, { value: "dark", label: "Dark" }]}
                            onChange={theme => onChange({ ...draft, theme: theme as HostPreferences["theme"] })}
                        />
                    </div>
                </div>
            </section>
        </fieldset>
        <section aria-labelledby="startup-settings">
            <h2 id="startup-settings">Startup</h2>
            <div class="group">
                <div class="row">
                    <div class="row-copy">
                        <span>Launch at login</span>
                        <p>Launch Nanika automatically when you sign in.</p>
                        {#if startupStatusMessage}<p>{startupStatusMessage}</p>{/if}
                        {#if startupError}<p class="error" role="alert">{startupError}</p>{/if}
                    </div>
                    <Switch
                        label="Launch at login"
                        checked={startup === "enabled"}
                        disabled={startup === null}
                        busy={startupBusy}
                        onChange={checked =>
                        {
                            void changeStartup(checked);
                        }}
                    />
                </div>
            </div>
        </section>
        <SettingsActions
            {dirty}
            {saving}
            {error}
            disabled={recording}
            onDiscard={() =>
            {
                onChange(structuredClone(preferences));
                error = null;
            }}
        />
    </form>
</div>

<style>
.general-page { display: flex; flex-direction: column; min-height: 100%; max-width: 52rem; margin: 0 auto; padding: 24px 28px 0; }
form { display: flex; flex: 1; flex-direction: column; min-height: 0; }
h1 { margin: 0 0 24px; font-size: 20px; font-weight: 600; }
h2 { margin: 0 0 9px; font-size: var(--font-control); font-weight: 600; }
section { margin-bottom: 24px; }
fieldset { margin: 0; padding: 0; border: 0; min-width: 0; }
.group { border: 1px solid var(--border-subtle); border-radius: var(--radius-row); background: var(--surface-form); }
.row { display: flex; align-items: center; justify-content: space-between; gap: 24px; padding: 14px 16px; font-size: 13px; }
.row + .row { border-top: 1px solid var(--border-subtle); }
.row-copy { min-width: 0; }
.row-copy p { margin: 4px 0 0; color: var(--text-secondary); font-size: var(--font-control); line-height: 1.5; }
.row-copy p.error { color: var(--text-danger); overflow-wrap: anywhere; }
@media (width < 800px) { .general-page { padding: 20px 20px 0; } }
</style>
