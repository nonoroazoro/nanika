<script lang="ts">
import Switch from "../components/Switch.svelte";
import Select from "../components/Select.svelte";
import ShortcutRecorder from "./ShortcutRecorder.svelte";
import SettingsField from "./SettingsField.svelte";
import type { SettingsState } from "./SettingsState.svelte";
import type { HostPreferences, StartupStatus } from "../types/Settings";

const { settings, startup, startupStatus }: {
    settings: SettingsState<HostPreferences>;
    startup: SettingsState<{ enabled: boolean; }> | null;
    startupStatus: StartupStatus | null;
} = $props();
const startupStatusMessage = $derived.by(() =>
{
    if (startupStatus === "requiresApproval")
    {
        return "Turn this on to open System Settings and allow Nanika.";
    }
    if (startupStatus === "needsRepair")
    {
        return "Turn this on to repair launch at login.";
    }
    return null;
});
</script>

<div class="general-page">
    <h1>General</h1>
    <div class="sections">
        <fieldset>
            <section aria-labelledby="launcher-settings">
                <h2 id="launcher-settings">Launcher</h2>
                <div class="group">
                    <div class="row">
                        <span id="shortcut-label">Open launcher</span>
                        <SettingsField
                            busy={settings.phase.get("launcherShortcut") !== undefined}
                            label="Open launcher"
                            startedAt={settings.startedAt.get("launcherShortcut")}
                        >
                            <ShortcutRecorder
                                value={settings.values.launcherShortcut}
                                registeredValue={settings.saved.launcherShortcut}
                                onChange={launcherShortcut => settings.change("launcherShortcut", launcherShortcut)}
                            />
                        </SettingsField>
                    </div>
                    <div class="row">
                        <span>Hide when focus is lost</span>
                        <SettingsField
                            busy={settings.phase.get("hideOnBlur") !== undefined}
                            label="Hide when focus is lost"
                            startedAt={settings.startedAt.get("hideOnBlur")}
                        >
                            <Switch
                                label="Hide when focus is lost"
                                checked={settings.values.hideOnBlur}
                                onChange={checked => settings.change("hideOnBlur", checked)}
                            />
                        </SettingsField>
                    </div>
                </div>
            </section>
            <section aria-labelledby="appearance-settings">
                <h2 id="appearance-settings">Appearance</h2>
                <div class="group">
                    <div class="row">
                        <span>Theme</span>
                        <SettingsField
                            busy={settings.phase.get("theme") !== undefined}
                            label="Theme"
                            startedAt={settings.startedAt.get("theme")}
                        >
                            <Select
                                label="Theme"
                                value={settings.values.theme}
                                options={[{ value: "system", label: "System" }, { value: "light", label: "Light" }, { value: "dark", label: "Dark" }]}
                                onChange={theme => settings.change("theme", theme as HostPreferences["theme"])}
                            />
                        </SettingsField>
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
                    </div>
                    <SettingsField
                        busy={startup?.phase.get("enabled") !== undefined}
                        label="Launch at login"
                        startedAt={startup?.startedAt.get("enabled")}
                    >
                        <Switch
                            label="Launch at login"
                            checked={startup?.values.enabled ?? false}
                            disabled={startup === null}
                            onChange={checked =>
                            {
                                void startup?.change("enabled", checked);
                            }}
                        />
                    </SettingsField>
                </div>
            </div>
        </section>
    </div>
</div>

<style>
.general-page { display: flex; flex-direction: column; min-height: 100%; max-width: 52rem; margin: 0 auto; padding: 24px 28px 0; }
.sections { display: flex; flex: 1; flex-direction: column; min-height: 0; }
h1 { margin: 0 0 24px; font-size: 20px; font-weight: 600; }
h2 { margin: 0 0 9px; font-size: var(--font-control); font-weight: 600; }
section { margin-bottom: 24px; }
fieldset { margin: 0; padding: 0; border: 0; min-width: 0; }
.group { border: 1px solid var(--border-subtle); border-radius: var(--radius-row); background: var(--surface-form); }
.row { display: flex; align-items: center; justify-content: space-between; gap: 24px; padding: 14px 16px; font-size: 13px; }
.row + .row { border-top: 1px solid var(--border-subtle); }
.row-copy { min-width: 0; }
.row-copy p { margin: 4px 0 0; color: var(--text-secondary); font-size: var(--font-control); line-height: 1.5; }
@media (width < 800px) { .general-page { padding: 20px 20px 0; } }
</style>
