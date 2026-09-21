<script lang="ts">
import { onMount, tick } from "svelte";

import { settingsBridge } from "./bridge/settingsBridge";
import SettingsValue from "./settings/SettingsValue.svelte";
import DirectoryList from "./settings/DirectoryList.svelte";
import GeneralSettings from "./settings/GeneralSettings.svelte";
import SettingsActions from "./settings/SettingsActions.svelte";
import { normalizeSettings } from "./settings/integerValue";
import ContributionIconTile from "./components/ContributionIconTile.svelte";
import type {
    ConfigurationValue,
    ExtensionSettings,
    HostPreferences,
    SettingsApplicationUpdate,
    SettingsSnapshot
} from "./types/Settings";

let snapshot = $state.raw<SettingsSnapshot | null>(null);
let hostDraft = $state.raw<HostPreferences | null>(null);
let drafts = $state.raw<Record<string, Record<string, ConfigurationValue>>>({});
let messages = $state.raw<Record<string, { text: string; failed: boolean; }>>({});
let selection = $state("general");
let loading = $state(true);
let loadError = $state(false);
let saving = $state(false);
const applicationResults: Record<string, SettingsApplicationUpdate> = {};
const extensions = $derived(snapshot?.extensions ?? []);
const selected = $derived(extensions.find(extension => extension.id === selection) ?? null);
const properties = $derived(Object.entries(selected?.configuration?.contribution.properties ?? {}));
const values = $derived(drafts[selection] ?? {});
const dirty = $derived(selected !== null && isDirty(selected));
const message = $derived(messages[selection]);
const hostDirty = $derived(snapshot !== null && JSON.stringify(hostDraft) !== JSON.stringify(snapshot.general));

onMount(() =>
{
    prepare();
});

function prepare(): void
{
    selection = "general";
    messages = {};
    for (const id of Object.keys(applicationResults))
    {
        delete applicationResults[id];
    }
    void load().then(async () =>
    {
        await tick();
        await settingsBridge.ready();
    }).catch(error =>
    {
        console.error("Settings could not finish opening", error);
        loadError = true;
    });
}

async function load(): Promise<void>
{
    loading = true;
    loadError = false;
    const buffered: SettingsApplicationUpdate[] = [];
    let receiving = false;
    try
    {
        snapshot = await settingsBridge.read(update =>
        {
            if (receiving)
            {
                recordApplication(update);
            }
            else
            {
                buffered.push(update);
            }
        }, prepare);
        hostDraft = structuredClone(snapshot.general);
        drafts = Object.fromEntries(
            snapshot.extensions.map(extension => [extension.id, structuredClone(extension.configuration.values)])
        );
        for (const extension of snapshot.extensions)
        {
            if (extension.application)
            {
                recordApplication(extension.application);
            }
        }
        // Channel messages can arrive before the initial command reply.
        for (const update of buffered)
        {
            recordApplication(update);
        }
        receiving = true;
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

function recordApplication(update: SettingsApplicationUpdate): void
{
    const current = applicationResults[update.extensionId];
    if (
        current && (current.requestId > update.requestId
            || (current.requestId === update.requestId && current.result.status !== "applying"))
    )
    {
        return;
    }
    applicationResults[update.extensionId] = update;
    const { result } = update;
    if (result.status === "applyFailed")
    {
        console.error(`Settings for ${update.extensionId} could not be applied`, result.error);
        messages = {
            ...messages,
            [update.extensionId]: {
                text: "Changes were saved but could not be applied.",
                failed: true
            }
        };
    }
    else
    {
        const next = { ...messages };
        delete next[update.extensionId];
        messages = next;
    }
}

function isDirty(extension: ExtensionSettings): boolean
{
    return JSON.stringify(
        normalizeSettings(extension.configuration.contribution.properties, drafts[extension.id] ?? {})
    ) !== JSON.stringify(extension.configuration.values);
}

function change(key: string, value: ConfigurationValue): void
{
    drafts = { ...drafts, [selection]: { ...values, [key]: value } };
    const next = { ...messages };
    delete next[selection];
    messages = next;
}

function discard(): void
{
    drafts = { ...drafts, [selection]: structuredClone(selected?.configuration?.values ?? {}) };
    const next = { ...messages };
    delete next[selection];
    messages = next;
}

async function save(event: SubmitEvent): Promise<void>
{
    event.preventDefault();
    if (saving || !selected || !dirty)
    {
        return;
    }
    const id = selected.id;
    const submitted = normalizeSettings(selected.configuration.contribution.properties, structuredClone(values));
    saving = true;
    try
    {
        const update = await settingsBridge.save(id, submitted);
        // Application can fail after persistence. Keep the saved baseline honest,
        // preserve the concrete error, and never resubmit automatically.
        if (snapshot)
        {
            snapshot = {
                ...snapshot,
                extensions: snapshot.extensions.map(extension =>
                    extension.id === id
                        ? { ...extension, configuration: { ...extension.configuration, values: submitted } }
                        : extension
                )
            };
        }
        recordApplication(update);
    }
    catch (error)
    {
        console.error(`Settings for ${id} could not be saved`, error);
        messages = {
            ...messages,
            [id]: { text: "Changes could not be saved. Try again.", failed: true }
        };
    }
    finally
    {
        saving = false;
    }
}
</script>

<main class="settings">
    <aside class="sidebar" aria-label="Settings navigation">
        <nav aria-label="Settings sections">
            <button
                type="button"
                class:active={selection === "general"}
                aria-current={selection === "general" ? "page" : undefined}
                onclick={() =>
                {
                    selection = "general";
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
                {#if hostDirty}<span class="unsaved-dot" aria-label="Unsaved changes"></span>{/if}
            </button>
            <h2>Extensions</h2>
            {#each extensions as extension (extension.id)}
                <button
                    type="button"
                    class:active={selection === extension.id}
                    aria-current={selection === extension.id ? "page" : undefined}
                    onclick={() =>
                    {
                        selection = extension.id;
                    }}
                >
                    <span class="nav-icon" aria-hidden="true"><ContributionIconTile kind={extension.icon} /></span>
                    <span class="nav-title">{extension.name}</span>
                    {#if isDirty(extension)}<span class="unsaved-dot" aria-label="Unsaved changes"></span>{/if}
                </button>
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
                <button
                    type="button"
                    onclick={() =>
                    {
                        void load();
                    }}
                >
                    Try again
                </button>
            </div>
        {:else if selection === "general" && snapshot && hostDraft}
            <GeneralSettings
                preferences={snapshot.general}
                draft={hostDraft}
                onChange={value =>
                {
                    hostDraft = value;
                }}
                onSaved={value =>
                {
                    if (snapshot)
                    {
                        snapshot = { ...snapshot, general: value };
                        hostDraft = structuredClone(value);
                    }
                }}
            />
        {:else if selected}
            <div class="extension-page">
                <header>
                    <div class="extension-heading">
                        <span class="heading-icon"><ContributionIconTile kind={selected.icon} /></span><div>
                            <h1>{selected.name}</h1>
                        </div>
                    </div>
                </header>
                {#key selected.id}
                    <form onsubmit={save}>
                        <fieldset disabled={saving} class="fields">
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
                                        {#if property.type === "array" && property.items?.format === "directory"}
                                            <DirectoryList
                                                paths={values[key] as string[]}
                                                maximum={property.maxItems ?? 0}
                                                label={property.title}
                                                titleId={`title-${key}`}
                                                description={property.description}
                                                onPick={() => settingsBridge.pickDirectory(selection, key)}
                                                onChange={paths => change(key, paths)}
                                            />
                                        {:else}
                                            <SettingsValue
                                                schema={property}
                                                value={values[key]}
                                                label={property.title}
                                                id={`setting-${key}`}
                                                onChange={value => change(key, value)}
                                            />
                                        {/if}
                                    </div>
                                </section>
                            {/each}
                        </fieldset>
                        <SettingsActions {dirty} {saving} error={message?.text} onDiscard={discard} />
                    </form>
                {/key}
            </div>
        {/if}
    </section>
</main>

<style>
.settings { display: grid; grid-template-columns: 13rem minmax(0, 1fr); width: 100%; height: 100%; background: var(--surface-window); color: var(--text-primary); font-size: var(--font-meta); }
.sidebar { display: flex; flex-direction: column; min-height: 0; gap: 18px; padding: 20px 10px 12px; border-right: 1px solid var(--border-subtle); background: var(--surface-hovered); }
nav { display: flex; flex: 1; min-height: 0; flex-direction: column; gap: 2px; overflow-y: auto; }
nav button { justify-content: flex-start; gap: 9px; flex: 0 0 auto; width: 100%; min-height: 34px; border: 0; border-radius: 6px; padding: 6px 10px; background: transparent; text-align: left; font-size: 13px; }
nav button.active { background: var(--surface-selected); font-weight: 600; }
.extension-heading { display: flex; align-items: center; gap: 12px; }
.heading-icon { --icon-size: 36px; display: block; flex-shrink: 0; }
.nav-icon { --icon-size: 22px; display: grid; width: 20px; height: 20px; flex-shrink: 0; place-items: center; color: var(--text-secondary); }
.nav-title { overflow: hidden; text-overflow: ellipsis; white-space: nowrap; }
nav h2 { display: flex; justify-content: space-between; margin: 22px 10px 6px; color: var(--text-secondary); font-size: 11px; font-weight: 600; }
.unsaved-dot { width: 5px; height: 5px; flex-shrink: 0; margin-left: auto; border-radius: 50%; background: var(--text-secondary); }
.sidebar-footer { padding: 8px 10px; color: var(--text-tertiary); font-size: 11px; }
.content { min-width: 0; min-height: 0; overflow-y: auto; }
.extension-page { max-width: 52rem; margin: 0 auto; padding: 24px 28px 0; }
.extension-page { display: flex; flex-direction: column; min-height: 100%; }
header { margin-bottom: 24px; }
h1 { margin: 0 0 5px; font-size: 20px; line-height: 1.35; letter-spacing: -0.015em; font-weight: 600; }
p { color: var(--text-secondary); font-size: 12px; line-height: 1.5; }
h2 { margin: 0 0 9px; font-size: 12px; font-weight: 600; }
form { display: flex; flex: 1; min-height: 0; flex-direction: column; }
.fields { display: grid; gap: 0; min-width: 0; margin: 0 0 24px; border: 1px solid var(--border-subtle); border-radius: 8px; padding: 0; background: var(--surface-form); }
.property { min-width: 0; padding: 16px; }
.property + .property { border-top: 1px solid var(--border-subtle); }
.property.scalar { display: grid; grid-template-columns: minmax(0, 1fr) auto; align-items: center; gap: 24px; }
.property h2 { margin: 0; font-size: 13px; font-weight: 500; }
.property-copy p { margin: 4px 0 0; }
.property:not(.scalar):not(.directory) .property-control { margin-top: 12px; }
.empty-state { display: grid; min-height: 100%; place-content: center; justify-items: center; gap: 12px; padding: 24px; text-align: center; }
button:focus-visible { background: var(--surface-selected); }
@media (width < 800px) { .settings { grid-template-columns: 12rem minmax(0, 1fr); } .extension-page { padding: 20px 20px 0; } }
</style>
