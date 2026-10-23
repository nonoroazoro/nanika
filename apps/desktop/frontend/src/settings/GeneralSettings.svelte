<script lang="ts">
import { localization } from "../localization/state.svelte";
import Switch from "../components/ui/Switch.svelte";
import Select from "../components/ui/Select.svelte";
import ShortcutRecorder from "./ShortcutRecorder.svelte";
import SettingsField from "./SettingsField.svelte";
import type { SettingsState } from "./SettingsState.svelte";
import type { LauncherSettings } from "./LauncherSettings";
import type { StartupSettings } from "./StartupSettings.svelte";
import { operationErrorText } from "../localization/operation-errors";
import Button from "../components/ui/Button.svelte";

import type { GeneralSettingsSection } from "../generated/GeneralSettingsSection";
import { settingsAnchor } from "./anchor";

const { settings, startup, sections, onStartupRetry }: {
    settings: SettingsState<LauncherSettings>;
    sections: GeneralSettingsSection[];
    startup: StartupSettings;
    onStartupRetry: () => void;
} = $props();
const startupStatusMessage = $derived.by(() =>
{
    if (startup.status === "requiresApproval")
    {
        return localization.text({ message: "settings-startup-approval" });
    }
    if (startup.status === "needsRepair")
    {
        return localization.text({ message: "settings-startup-repair" });
    }
    return null;
});
</script>

<div class="general-page" id={settingsAnchor("general", { kind: "page" })}>
    <h1>{localization.text({ message: "settings-general" })}</h1>
    <div class="sections">
        {#each sections as section (section.key)}
            {@const sectionId = settingsAnchor("general", { kind: "section", key: section.key })}
            <section id={sectionId} aria-labelledby={`${sectionId}/label`}>
                <h2 id={`${sectionId}/label`}>{localization.text(section.title)}</h2>
                <div class="group settings-search-target">
                    {#each section.fields as field (field.key)}
                        {@const fieldId = settingsAnchor("general", { kind: "field", key: field.key })}
                        <div
                            class="row settings-search-target"
                            id={fieldId}
                        >
                            <div class="row-copy">
                                <span>{localization.text(field.title)}</span>
                                {#if field.description}<p>{localization.text(field.description)}</p>{/if}
                                {#if field.key === "launchAtLogin" && startupStatusMessage}<p>
                                        {startupStatusMessage}
                                    </p>{/if}
                                {#if field.key === "launchAtLogin" && startup.readError}
                                    <p class="startup-error" role="alert">
                                        {
                                            operationErrorText(startup.readError, localization.locale, value => localization.text(value))
                                        }
                                    </p>
                                    <Button
                                        disabled={startup.reading}
                                        aria-busy={startup.reading}
                                        onclick={onStartupRetry}
                                    >
                                        {localization.text({ message: "action-retry" })}
                                    </Button>
                                {/if}
                            </div>
                            {#if field.key === "launchAtLogin"}
                                <SettingsField
                                    busy={startup.settings.phase.get("enabled") !== undefined}
                                    label={localization.text(field.title)}
                                    startedAt={startup.settings.startedAt.get("enabled")}
                                >
                                    <Switch
                                        label={localization.text(field.title)}
                                        checked={startup.settings.values.enabled}
                                        disabled={startup.status === null}
                                        onChange={checked =>
                                        {
                                            void startup.settings.change("enabled", checked);
                                        }}
                                    />
                                </SettingsField>
                            {:else}
                                <SettingsField
                                    busy={settings.phase.get(field.key) !== undefined}
                                    disableWhileBusy={field.key !== "language"}
                                    label={localization.text(field.title)}
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
                                            label={localization.text(field.title)}
                                            checked={settings.values.hideOnBlur}
                                            onChange={checked => settings.change("hideOnBlur", checked)}
                                        />
                                    {:else if field.key === "language"}
                                        <Select
                                            label={localization.text(field.title)}
                                            value={settings.values.language}
                                            pending={settings.phase.has("language")}
                                            options={[{ value: "system", label: localization.text({ message: "settings-language-system" }) }, {
                                                value: "en",
                                                label: "English"
                                            }, { value: "zh-Hans", label: "简体中文" }]}
                                            onChange={language => settings.change("language", language as LauncherSettings["language"])}
                                        />
                                    {:else if field.key === "theme"}
                                        <Select
                                            label={localization.text(field.title)}
                                            value={settings.values.theme}
                                            options={[{ value: "system", label: localization.text({ message: "settings-theme-system" }) }, {
                                                value: "light",
                                                label: localization.text({ message: "settings-theme-light" })
                                            }, { value: "dark", label: localization.text({ message: "settings-theme-dark" }) }]}
                                            onChange={theme => settings.change("theme", theme as LauncherSettings["theme"])}
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
.row-copy .startup-error { color: var(--text-danger); white-space: pre-wrap; overflow-wrap: anywhere; }
.row-copy p { margin: 4px 0 0; color: var(--text-secondary); font-size: var(--settings-description-size); line-height: var(--settings-description-line-height); }
@media (width < 800px) { .general-page { padding: var(--space-4) var(--space-4) 0; } }
</style>
