<script lang="ts">
import Button from "./components/Button.svelte";
import { onMount, tick } from "svelte";

import { settingsBridge } from "./bridge/settingsBridge";
import SettingsValue from "./settings/SettingsValue.svelte";
import DirectoryList from "./settings/DirectoryList.svelte";
import GeneralSettings from "./settings/GeneralSettings.svelte";
import SettingsToast from "./settings/SettingsToast.svelte";
import SettingsField from "./settings/SettingsField.svelte";
import { SettingsApplications } from "./settings/SettingsApplications";
import { SettingsState } from "./settings/SettingsState.svelte";
import { StartupSettings } from "./settings/StartupSettings.svelte";
import { uiActivity } from "./ui/activity";
import { prepareSetting } from "./settings/validation";
import SettingsTitleBar from "./settings/SettingsTitleBar.svelte";
import type { SettingsWindowAction } from "./types/SettingsWindowAction";
import { orderedProperties } from "./settings/properties";
import ContributionIconTile from "./components/ContributionIconTile.svelte";
import type {
    ConfigurationValue,
    HostPreferences,
    SettingsApplicationUpdate,
    SettingsSnapshot
} from "./types/Settings";

let notification = $state<string | null>(null);
let customControls = $state(false);
let maximized = $state(false);
let windowError = $state<string | null>(null);
let snapshot = $state.raw<SettingsSnapshot | null>(null);
let host = $state.raw<SettingsState<HostPreferences> | null>(null);
const startup = new StartupSettings(settingsBridge.readStartup, settingsBridge.setStartup, _notify);
let states = $state.raw<Record<string, SettingsState<Record<string, ConfigurationValue>>>>({});
let selection = $state("general");
let loading = $state(true);
let loadError = $state(false);
const applications = new SettingsApplications();
const extensions = $derived(snapshot?.extensions ?? []);
const selected = $derived(extensions.find(extension => extension.id === selection) ?? null);
const properties = $derived(orderedProperties(selected?.configuration?.contribution.properties ?? {}));
const selectedState = $derived(states[selection]);
const values = $derived(selectedState?.values ?? {});

onMount(() =>
{
    _prepare();
    void _loadStartup();
    let active = false;
    return uiActivity.subscribe(activity =>
    {
        const next = activity.visible && activity.focused;
        if (next && !active)
        {
            void _loadStartup();
        }
        active = next;
    });
});

function _prepare(): void
{
    windowError = null;
    selection = "general";
    void _load().then(async () =>
    {
        await tick();
        customControls = await settingsBridge.ready();
    }).catch(error =>
    {
        console.error("Settings could not finish opening", error);
        loadError = true;
    });
}

async function _load(): Promise<void>
{
    loading = true;
    loadError = false;
    try
    {
        snapshot = await settingsBridge.read(
            _application,
            _closed,
            next =>
            {
                maximized = next;
            },
            error => console.error("Settings progress receipt failed", error)
        );
        maximized = snapshot.maximized;
        host = new SettingsState(
            { values: snapshot.general, saved: snapshot.general, effective: snapshot.general, error: null },
            settingsBridge.saveHost,
            undefined,
            _notify
        );
        // Snapshot replies and Channel updates share the same ordering rules.
        // A delayed progress message must not replace a terminal result during loading.
        for (const extension of snapshot.extensions)
        {
            if (extension.application)
            {
                applications.record(extension.application);
            }
        }
        states = Object.fromEntries(snapshot.extensions.flatMap(extension =>
        {
            const configuration = extension.configuration;
            if (!configuration)
            {
                return [];
            }
            const schemas = configuration.contribution.properties;
            const application = applications.current(extension.id);
            return [[
                extension.id,
                new SettingsState(
                    application?.result.status === "completed"
                        ? application.result
                        : { ...configuration, error: null },
                    async (key, value) =>
                    {
                        const update = await settingsBridge.save(extension.id, key, value);
                        return await applications.completion(update);
                    },
                    (key, draft) => prepareSetting(schemas, key, draft),
                    _notify
                )
            ]];
        }));
        for (const extension of snapshot.extensions)
        {
            const application = applications.current(extension.id);
            const state = states[extension.id];
            if (state && application?.result.status === "running")
            {
                if (application.result.progress)
                {
                    state.progress.set(application.key, application.result.progress);
                }
                state.resume(application.key, applications.completion(application));
            }
        }
    }
    catch (error)
    {
        console.error("Settings could not load", error);
        loadError = true;
    }
    finally
    {
        loading = false;
    }
}

async function _loadStartup(): Promise<void>
{
    try
    {
        await startup.refresh();
    }
    catch (error)
    {
        console.error("Startup settings could not load", error);
    }
}

function _application(update: SettingsApplicationUpdate): void
{
    if (applications.record(update) && update.result.status === "running" && update.result.progress)
    {
        const state = states[update.extensionId];
        if (state)
        {
            state.progress.set(update.key, update.result.progress);
        }
    }
}

function _notify(error: string): void
{
    notification = error;
}

function _closed(): void
{
    // Retain pending writes and failed edits across native closes; reloading can overwrite them.
    selectedState?.commitEdits();
    selection = "general";
    void tick().then(() => settingsBridge.ready()).catch(error =>
    {
        console.error("Settings could not finish closing", error);
        windowError = String(error);
    });
}

function _windowAction(action: SettingsWindowAction): void
{
    void settingsBridge.windowAction(action).catch(error =>
    {
        windowError = String(error);
    });
}
</script>

<div class="settings-window" class:maximized>
    <SettingsToast
        message={notification}
        onDismiss={() =>
        {
            notification = null;
        }}
    />
    {#if customControls}<SettingsTitleBar {maximized} onAction={_windowAction} />{/if}
    {#if windowError}<div class="window-error" role="alert">{windowError}</div>{/if}
    <main class="settings">
        <aside class="sidebar" aria-label="Settings navigation">
            <nav aria-label="Settings sections">
                <Button
                    class={{ active: selection === "general" }}
                    aria-current={selection === "general" ? "page" : undefined}
                    onclick={() =>
                    {
                        selectedState?.commitEdits();
                        selection = "general";
                        void _loadStartup();
                    }}
                >
                    <span class="nav-icon" aria-hidden="true"><svg
                            width="18"
                            height="18"
                            viewBox="0 0 24 24"
                            fill="none"
                            stroke="currentColor"
                            stroke-width="1.6"
                        >
                            <rect x="3" y="4" width="18" height="16" rx="3" />
                            <path d="M3 10h18M9 10v10" />
                        </svg></span>General
                </Button>
                <h2>Extensions</h2>
                {#each extensions as extension (extension.id)}
                    <Button
                        class={{ active: selection === extension.id }}
                        aria-current={selection === extension.id ? "page" : undefined}
                        onclick={() =>
                        {
                            selectedState?.commitEdits();
                            selection = extension.id;
                        }}
                    >
                        <span class="nav-icon" aria-hidden="true"><ContributionIconTile kind={extension.icon} /></span>
                        <span class="nav-title">{extension.name}</span>
                    </Button>
                {/each}
            </nav>
            <div class="sidebar-footer">Nanika {snapshot?.version ?? ""}</div>
        </aside>
        <section class="content" aria-label="Settings content" aria-busy={loading}>
            {#if loading}
                <div class="empty-state" role="status">Loading settings…</div>
            {:else if loadError}
                <div class="empty-state" role="alert">
                    <h1>Settings could not load</h1>
                    <Button
                        onclick={() =>
                        {
                            void _load();
                        }}
                    >
                        Try again
                    </Button>
                </div>
            {:else if selection === "general" && host}
                <GeneralSettings
                    settings={host}
                    startup={startup.status === null ? null : startup.settings}
                    startupStatus={startup.status}
                />
            {:else if selected}
                <div class="extension-page">
                    <header>
                        <div class="extension-heading">
                            <span class="heading-icon"><ContributionIconTile kind={selected.icon} /></span><div>
                                <h1>{selected.name}</h1>
                                {#if !selected.enabled}<p>Disabled</p>{/if}
                            </div>
                        </div>
                    </header>
                    {#if selected.configurationError}
                        <p role="alert">{selected.configurationError}</p>
                    {:else if properties.length === 0}
                        <p>No settings available for this extension.</p>
                    {:else}
                        {#key selected.id}
                            <form
                                onsubmit={event =>
                                {
                                    event.preventDefault();
                                    selectedState?.commitEdits();
                                }}
                            >
                                <div class="fields">
                                    {#each properties as [key, property] (key)}
                                        <section
                                            class="property"
                                            class:directory={property.type === "array" && property.items?.format === "directory"}
                                            class:scalar={property.type === "boolean" || property.type === "integer"}
                                            aria-labelledby={`title-${key}`}
                                        >
                                            {#if !(property.type === "array" && property.items?.format === "directory")}<div class="property-copy">
                                                    <h2 id={`title-${key}`}>{property.title}</h2>
                                                    {#if property.description}<p>{property.description}</p>{/if}
                                                </div>{/if}
                                            <div class="property-control">
                                                <SettingsField
                                                    busy={selectedState?.phase.get(key) !== undefined}
                                                    label={property.title}
                                                    startedAt={selectedState?.startedAt.get(key)}
                                                    progress={selectedState?.progress.get(key)}
                                                    onCommit={() =>
                                                    {
                                                        void selectedState?.commit(key);
                                                    }}
                                                >
                                                    {#if property.type === "array" && property.items?.format === "directory"}
                                                        <DirectoryList
                                                            paths={values[key] as string[]}
                                                            maximum={property.maxItems ?? 0}
                                                            label={property.title}
                                                            titleId={`title-${key}`}
                                                            description={property.description}
                                                            onPick={() => settingsBridge.pickDirectory(selection, key)}
                                                            onChange={paths =>
                                                            {
                                                                void selectedState?.change(key, paths);
                                                            }}
                                                        />
                                                    {:else}
                                                        <SettingsValue
                                                            schema={property}
                                                            value={values[key]}
                                                            label={property.title}
                                                            id={`setting-${key}`}
                                                            onChange={value => selectedState?.edit(key, value)}
                                                            onCommit={() =>
                                                            {
                                                                void selectedState?.commit(key);
                                                            }}
                                                        />
                                                    {/if}
                                                </SettingsField>
                                            </div>
                                        </section>
                                    {/each}
                                </div>
                            </form>
                        {/key}
                    {/if}
                </div>
            {/if}
        </section>
    </main>
</div>

<style>
.settings-window { position: relative; display: flex; flex-direction: column; width: 100%; height: 100%; overflow: hidden; border: 1px solid var(--border-subtle); border-radius: var(--radius-window); background: var(--surface-window); }
.settings-window.maximized { border: 0; border-radius: 0; }
.window-error { padding: var(--space-2) var(--space-5); color: var(--text-danger); font-size: var(--font-control); }

.settings { display: grid; min-height: 0; flex: 1; grid-template-columns: 13rem minmax(0, 1fr); width: 100%; height: 100%; background: var(--surface-window); color: var(--text-primary); font-size: var(--font-meta); }
.sidebar { display: flex; flex-direction: column; min-height: 0; gap: 18px; padding: 20px 10px 12px; border-right: 1px solid var(--border-subtle); background: var(--surface-hovered); }
nav { display: flex; flex: 1; min-height: 0; flex-direction: column; gap: 2px; overflow-y: auto; }
nav :global(button) { justify-content: flex-start; gap: 9px; flex: 0 0 auto; width: 100%; min-height: 34px; border: 0; border-radius: var(--control-radius); padding: 6px 10px; background: transparent; text-align: left; font-size: 13px; }
nav :global(button.active) { background: var(--surface-selected); font-weight: 600; }
.extension-heading { display: flex; align-items: center; gap: 12px; }
.heading-icon { --icon-size: 36px; display: block; flex-shrink: 0; }
.nav-icon { --icon-size: 22px; display: grid; width: 20px; height: 20px; flex-shrink: 0; place-items: center; color: var(--text-secondary); }
.nav-title { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
nav h2 { display: flex; justify-content: space-between; margin: 22px 10px 6px; color: var(--text-secondary); font-size: 11px; font-weight: 600; }
.sidebar-footer { padding: 8px 10px; color: var(--text-tertiary); font-size: 11px; }
.content { min-width: 0; min-height: 0; overflow-y: auto; }
.extension-page { max-width: 52rem; margin: 0 auto; padding: 24px 28px 0; }
.extension-page { display: flex; flex-direction: column; min-height: 100%; }
header { margin-bottom: 24px; }
h1 { margin: 0 0 5px; font-size: 20px; line-height: 1.35; letter-spacing: -0.015em; font-weight: 600; }
p { color: var(--text-secondary); font-size: var(--font-control); line-height: 1.5; }
h2 { margin: 0 0 9px; font-size: var(--font-control); font-weight: 600; }
form { display: flex; flex: 1; min-height: 0; flex-direction: column; }
.fields { display: grid; gap: 0; min-width: 0; margin: 0 0 24px; border: 1px solid var(--border-subtle); border-radius: var(--radius-row); padding: 0; background: var(--surface-form); }
.property { min-width: 0; padding: 16px; }
.property + .property { border-top: 1px solid var(--border-subtle); }
.property.scalar { display: grid; grid-template-columns: minmax(0, 1fr) auto; align-items: center; gap: 24px; }
.property h2 { margin: 0; font-size: 13px; font-weight: 500; }
.property-copy p { margin: 4px 0 0; }
.property:not(.scalar):not(.directory) .property-control { margin-top: 12px; }
.empty-state { display: grid; min-height: 100%; place-content: center; justify-items: center; gap: 12px; padding: 24px; text-align: center; }
@media (width < 800px) { .settings { grid-template-columns: 12rem minmax(0, 1fr); } .extension-page { padding: 20px 20px 0; } }
</style>
