<script lang="ts">
import Switch from "../components/ui/Switch.svelte";
import Select from "../components/ui/Select.svelte";
import ShortcutRecorder from "./ShortcutRecorder.svelte";
import SettingsField from "./SettingsField.svelte";
import type { SettingsState } from "./SettingsState.svelte";
import type { HostPreferences, StartupStatus } from "../types/Settings";

import type { GeneralSettingsSection } from "../types/GeneralSettingsSection";
import { settingsAnchor } from "./anchor";

const { settings, startup, startupStatus, sections }: {
    settings: SettingsState<HostPreferences>;
    sections: GeneralSettingsSection[];
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

<div class="general-page" id={settingsAnchor("general", { kind: "page" })}>
    <h1>General</h1>
    <div class="sections">
        {#each sections as section (section.key)}
            {@const sectionId = settingsAnchor("general", { kind: "section", key: section.key })}
            <section id={sectionId} aria-labelledby={`${sectionId}/label`}>
                <h2 id={`${sectionId}/label`}>{section.title}</h2>
                <div class="group settings-search-target">
                    {#each section.fields as field (field.key)}
                        {@const fieldId = settingsAnchor("general", { kind: "field", key: field.key })}
                        <div
                            class="row settings-search-target"
                            id={fieldId}
                        >
                            <div class="row-copy">
                                <span>{field.title}</span>
                                {#if field.description}<p>{field.description}</p>{/if}
                                {#if field.key === "launchAtLogin" && startupStatusMessage}<p>
                                        {startupStatusMessage}
                                    </p>{/if}
                            </div>
                            {#if field.key === "launchAtLogin"}
                                <SettingsField
                                    busy={startup?.phase.get("enabled") !== undefined}
                                    label={field.title}
                                    startedAt={startup?.startedAt.get("enabled")}
                                >
                                    <Switch
                                        label={field.title}
                                        checked={startup?.values.enabled ?? false}
                                        disabled={startup === null}
                                        onChange={checked =>
                                        {
                                            void startup?.change("enabled", checked);
                                        }}
                                    />
                                </SettingsField>
                            {:else}
                                <SettingsField
                                    busy={settings.phase.get(field.key) !== undefined}
                                    label={field.title}
                                    startedAt={settings.startedAt.get(field.key)}
                                >
                                    {#if field.key === "launcherShortcut"}
                                        <ShortcutRecorder
                                            value={settings.values.launcherShortcut}
                                            registeredValue={settings.saved.launcherShortcut}
                                            onChange={value => settings.change("launcherShortcut", value)}
                                        />
                                    {:else if field.key === "hideOnBlur"}
                                        <Switch
                                            label={field.title}
                                            checked={settings.values.hideOnBlur}
                                            onChange={checked => settings.change("hideOnBlur", checked)}
                                        />
                                    {:else if field.key === "theme"}
                                        <Select
                                            label={field.title}
                                            value={settings.values.theme}
                                            options={[{ value: "system", label: "System" }, { value: "light", label: "Light" }, { value: "dark", label: "Dark" }]}
                                            onChange={theme => settings.change("theme", theme as HostPreferences["theme"])}
                                        />
                                    {/if}
                                </SettingsField>
                            {/if}
                        </div>
                    {/each}
                </div>
            </section>
        {/each}
    </div>
</div>

<style>
.general-page { display: flex; flex-direction: column; min-height: 100%; max-width: 52rem; margin: 0 auto; padding: var(--space-4) var(--space-6) 0; }
.sections { display: flex; flex: 1; flex-direction: column; min-height: 0; }
h1 { margin: 0 0 var(--settings-group-gap); font-size: 20px; line-height: 26px; font-weight: var(--settings-heading-weight); }
h2 { color: var(--text-secondary); margin: 0 0 var(--space-2); font-size: var(--settings-description-size); line-height: var(--settings-description-line-height); font-weight: var(--settings-heading-weight); }
section { margin-bottom: var(--settings-group-gap); }
.group { border: 1px solid var(--border-subtle); border-radius: var(--radius-row); background: var(--surface-form); }
.row { display: flex; align-items: center; justify-content: space-between; gap: var(--space-4); padding: var(--settings-row-padding); font-size: var(--settings-label-size); line-height: var(--settings-label-line-height); }
.row + .row { border-top: 1px solid var(--border-subtle); }
.row-copy { min-width: 0; }
.row-copy p { margin: 4px 0 0; color: var(--text-secondary); font-size: var(--settings-description-size); line-height: var(--settings-description-line-height); }
@media (width < 800px) { .general-page { padding: var(--space-4) var(--space-4) 0; } }
</style>
