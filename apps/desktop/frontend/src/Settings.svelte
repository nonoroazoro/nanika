<script lang="ts">
import { ExtensionPresentation } from "./localization/ExtensionPresentation";
import { ProgressPresentation } from "./localization/ProgressPresentation";
import { ResourceCache } from "./localization/ResourceCache";
import type { ExtensionResourceScope } from "./generated/ExtensionResourceScope";
import type { LocalizedText } from "./generated/LocalizedText";
import type { TargetPlatform } from "./generated/TargetPlatform";
import { localization } from "./localization/state.svelte";
import { lifecycleErrorText } from "./localization/lifecycle-errors";
import { operationErrorText, operationFailure } from "./localization/operation-errors";
import type { OperationError } from "./generated/OperationError";
import { settingsFailure } from "./settings/error-presentation";
import type { LocalizationSnapshot } from "./generated/LocalizationSnapshot";
import { isCompositionKey } from "./ui/keyboard";
import "./settings/fields.css";
import SettingsSidebar from "./settings/SettingsSidebar.svelte";
import { SettingsSearchState } from "./settings/SettingsSearchState.svelte";
import { settingsAnchor } from "./settings/anchor";
import { revealSetting } from "./settings/reveal";
import type { SettingsSearchTarget } from "./generated/SettingsSearchTarget";
import ScrollArea from "./components/ui/ScrollArea.svelte";
import SelectionHighlight from "./components/motion/SelectionHighlight.svelte";
import "./styles/settings.css";
import Switch from "./components/ui/Switch.svelte";
import type { ExtensionLifecycle } from "./generated/ExtensionLifecycle";
import { SvelteMap } from "svelte/reactivity";
import Button from "./components/ui/Button.svelte";
import { onMount, tick } from "svelte";

import { settingsBridge } from "./bridge/settingsBridge";
import SettingsActions from "./settings/SettingsActions.svelte";
import type { SettingsNavigation } from "./generated/SettingsNavigation";
import SettingsValue from "./settings/SettingsValue.svelte";
import DirectoryList from "./settings/DirectoryList.svelte";
import GeneralSettings from "./settings/GeneralSettings.svelte";
import AboutSettings from "./settings/AboutSettings.svelte";
import SettingsToast from "./settings/SettingsToast.svelte";
import SettingsField from "./settings/SettingsField.svelte";
import ExtensionStatus from "./settings/ExtensionStatus.svelte";
import type { SettingsError } from "./settings/SettingsError";
import { SettingsApplications } from "./settings/SettingsApplications";
import { SettingsState } from "./settings/SettingsState.svelte";
import { StartupSettings } from "./settings/StartupSettings.svelte";
import { uiActivity } from "./ui/activity";
import { ExtensionSettingsState } from "./settings/ExtensionSettingsState.svelte";
import SettingsTitleBar from "./settings/SettingsTitleBar.svelte";
import type { SettingsWindowAction } from "./generated/SettingsWindowAction";
import { propertyGroups } from "./settings/properties";
import ExtensionIcon from "./components/extensions/ExtensionIcon.svelte";
import type { ExtensionSettings } from "./generated/ExtensionSettings";
import type { LauncherSettings } from "./settings/LauncherSettings";
import type { SettingsApplicationUpdate } from "./generated/SettingsApplicationUpdate";
import type { SettingsSnapshot } from "./generated/SettingsSnapshot";

$effect(() =>
{
    document.documentElement.lang = localization.locale;
});

let notification = $state<SettingsError | null>(null);
let customControls = $state(false);
let maximized = $state(false);
let windowError = $state<OperationError | null>(null);
let snapshot = $state.raw<SettingsSnapshot | null>(null);
const resources = new ResourceCache(settingsBridge.readResources);
let presentations = $state.raw(new Map<string, ExtensionPresentation>());
let presentationSession: number | null = null;
let presentationError = $state<OperationError | null>(null);
let preparingPresentation = $state(false);
let presentationAttempt = 0;
let host = $state.raw<SettingsState<LauncherSettings> | null>(null);
const startup = new StartupSettings(settingsBridge.readStartup, settingsBridge.setStartup, _notify);
let states = $state.raw<Record<string, ExtensionSettingsState>>({});
let selection = $state("general");
let loading = $state(true);
let loadError = $state<OperationError | null>(null);
const applications = new SettingsApplications();
const search = new SettingsSearchState(settingsBridge.search);
let revealed = $state<string | null>(null);
let navigationRevision = 0;
let requestedNavigation: SettingsNavigation | null = null;
let detailViewport = $state<HTMLDivElement | null>(null);
let detailContent = $state<HTMLElement | null>(null);
let highlightedTarget = $state<HTMLElement | null>(null);
let searchInput = $state<HTMLInputElement>();
// The supported native adapters use custom caption controls only on Windows.
const searchShortcut = $derived(customControls ? "Control+F" : "Meta+F");
const toggling = new SvelteMap<string, number>();
let lifecycleRevision = 0;
let lifecycle: ExtensionLifecycle[] = [];
const extensions = $derived(snapshot?.extensions ?? []);
const selected = $derived(extensions.find(extension => extension.id === selection) ?? null);
const groups = $derived(
    propertyGroups(
        selected?.configuration?.contribution.properties ?? {},
        selected?.configuration?.contribution.title ?? ""
    )
);
const selectedState = $derived(states[selection]);
const values = $derived(selectedState?.values ?? {});

onMount(() =>
{
    _prepare();
    void _loadStartup();
    let active = false;
    return uiActivity.subscribe(activity =>
    {
        search.setVisible(activity.visible);
        const next = activity.visible && activity.focused;
        if (next && !active)
        {
            void _loadStartup();
        }
        active = next;
    });
});

function _localization(sessionId: number, next: LocalizationSnapshot): void
{
    if (!localization.accept(sessionId, next) || !host)
    {
        return;
    }
    // Reconcile after accepted writes; SettingsState preserves unrelated drafts and caret state.
    const current = host;
    current.observe(() =>
    {
        return {
            values: { ...current.saved, language: localization.preference },
            saved: { ...current.saved, language: localization.preference },
            effective: current.effective ? { ...current.effective, language: localization.preference } : null,
            error: null
        };
    });
}

function _requestedNavigation(next: SettingsNavigation): void
{
    if (requestedNavigation && next.revision <= requestedNavigation.revision)
    {
        return;
    }
    requestedNavigation = next;
    search.update("");
    if (!loading)
    {
        void _navigate(next.extensionId ?? "general", { kind: "page" });
    }
}

function _prepare(): void
{
    windowError = null;
    selection = "general";
    void _load().then(_ready).catch(error =>
    {
        console.error("Settings could not finish opening", error);
        loadError = operationFailure(error, "settingsReadFailed");
    });
}

async function _ready(): Promise<void>
{
    windowError = null;
    try
    {
        await tick();
        customControls = await settingsBridge.ready();
    }
    catch (error)
    {
        console.error("Settings window could not finish updating", error);
        windowError = operationFailure(error, "settingsReadyFailed");
    }
}

async function _load(): Promise<void>
{
    loading = true;
    loadError = null;
    try
    {
        const loaded = await settingsBridge.read(
            _application,
            _closed,
            next =>
            {
                maximized = next;
            },
            error => console.error("Settings progress receipt failed", error),
            _lifecycle,
            _requestedNavigation,
            _localization
        );
        // Host messages must use the saved locale even when package preparation fails.
        localization.initialize(loaded.sessionId, loaded.localization, loaded.platform);
        // Prepare immutable package names before exposing metadata to the sidebar or search.
        const prepared = await _presentations(loaded.extensions, loaded.sessionId, loaded.platform);
        loaded.general.language = localization.preference;
        presentations = prepared;
        presentationSession = loaded.sessionId;
        if (lifecycleRevision <= loaded.lifecycleRevision)
        {
            lifecycleRevision = loaded.lifecycleRevision;
            lifecycle = loaded.extensions;
        }
        snapshot = loaded;
        _requestedNavigation(snapshot.navigation);
        maximized = snapshot.maximized;
        host = new SettingsState<LauncherSettings>(
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
        states = {};
        for (const extension of snapshot.extensions)
        {
            _observeConfiguration(extension);
        }
        await _refreshProgress();
        if (lifecycleRevision > snapshot.lifecycleRevision)
        {
            await _mergeLifecycle();
        }
    }
    catch (error)
    {
        console.error("Settings could not load", error);
        loadError = operationFailure(error, "settingsReadFailed");
    }
    finally
    {
        loading = false;
        if (requestedNavigation)
        {
            void _navigate(requestedNavigation.extensionId ?? "general", { kind: "page" });
        }
        search.refresh();
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

async function _application(update: SettingsApplicationUpdate): Promise<void>
{
    if (applications.record(update))
    {
        await _prepareProgress(update);
    }
}

async function _prepareProgress(update: SettingsApplicationUpdate): Promise<void>
{
    const state = states[update.extensionId];
    const current = snapshot;
    if (!state || !current || update.result.status !== "running" || !update.result.progress)
    {
        return;
    }
    try
    {
        const scope = update.resourceScope;
        if (scope && scope.extensionId !== update.extensionId)
        {
            throw new Error("Progress resources do not belong to the operation owner.");
        }
        const retained = presentations.get(update.extensionId)?.resources;
        let bundle = scope ? retained : undefined;
        if (scope && bundle?.identity !== scope.identity)
        {
            bundle = await resources.load({ surface: "settings", sessionId: current.sessionId, scope });
        }
        // Terminal results bypass progress preparation and must never be replaced by its late completion.
        if (snapshot?.sessionId !== current.sessionId || applications.current(update.extensionId) !== update)
        {
            return;
        }
        state.progress.set(
            update.key,
            new ProgressPresentation(
                update.result.progress,
                bundle,
                current.platform,
                state.progress.get(update.key)
            )
        );
    }
    catch (error)
    {
        if (applications.current(update.extensionId) === update)
        {
            presentationError = operationFailure(error, "localizationReadFailed");
        }
    }
}

async function _refreshProgress(): Promise<void>
{
    for (const extension of snapshot?.extensions ?? [])
    {
        const update = applications.current(extension.id);
        if (update)
        {
            await _prepareProgress(update);
        }
    }
}

async function _lifecycle(revision: number, updates: ExtensionLifecycle[]): Promise<void>
{
    if (revision <= lifecycleRevision)
    {
        return;
    }
    lifecycleRevision = revision;
    lifecycle = updates;
    await _mergeLifecycle();
}

async function _mergeLifecycle(): Promise<void>
{
    if (!snapshot)
    {
        return;
    }
    const current = snapshot;
    const attempt = ++presentationAttempt;
    const revision = lifecycleRevision;
    const updatesToApply = lifecycle;
    preparingPresentation = true;
    try
    {
        const prepared = await _presentations(updatesToApply, current.sessionId, current.platform);
        if (
            snapshot?.sessionId !== current.sessionId || revision !== lifecycleRevision
            || attempt !== presentationAttempt
        )
        {
            return;
        }
        const updates = new Map(updatesToApply.map(extension => [extension.id, extension]));
        for (const extension of updatesToApply)
        {
            _observeConfiguration(extension);
        }
        presentations = prepared;
        snapshot = {
            ...snapshot,
            lifecycleRevision: revision,
            extensions: snapshot.extensions.map(extension => ({ ...extension, ...updates.get(extension.id) }))
        };
        presentationError = null;
        await _refreshProgress();
        search.refresh();
    }
    catch (error)
    {
        if (attempt === presentationAttempt)
        {
            presentationError = operationFailure(error, "localizationReadFailed");
        }
    }
    finally
    {
        if (attempt === presentationAttempt)
        {
            preparingPresentation = false;
        }
    }
}

async function _presentations(
    metadata: ExtensionLifecycle[],
    sessionId: number,
    platform: TargetPlatform
): Promise<Map<string, ExtensionPresentation>>
{
    const prepared = new SvelteMap<string, ExtensionPresentation>();
    for (const extension of metadata)
    {
        const previous = presentationSession === sessionId ? presentations.get(extension.id) : undefined;
        if (
            previous?.matches(
                extension.id,
                extension.name,
                extension.resourceScope,
                platform,
                extension.configuration?.contribution
            )
        )
        {
            prepared.set(extension.id, previous);
            continue;
        }
        const bundle = extension.resourceScope
            ? await resources.load({ surface: "settings", sessionId, scope: extension.resourceScope })
            : undefined;
        prepared.set(
            extension.id,
            new ExtensionPresentation(
                extension.id,
                extension.name,
                extension.resourceScope,
                bundle,
                platform,
                extension.configuration?.contribution
            )
        );
    }
    return prepared;
}

function _text(value: LocalizedText, scope?: ExtensionResourceScope): string
{
    if (!scope)
    {
        return localization.text(value);
    }
    const prepared = presentations.get(scope.extensionId);
    if (!prepared || prepared.resources?.identity !== scope.identity)
    {
        throw new Error("Settings text does not belong to its prepared package.");
    }
    return prepared.text(value, localization.locale);
}

function _observeConfiguration(extension: Pick<ExtensionSettings, "id" | "configuration">): void
{
    const configuration = extension.configuration;
    if (!configuration)
    {
        return;
    }
    const existing = states[extension.id];
    if (existing)
    {
        existing.observeConfiguration(configuration);
        return;
    }
    const application = applications.current(extension.id);
    const previous = application?.result;
    const state = new ExtensionSettingsState(
        configuration,
        previous && previous.status !== "running" ? previous.error : null,
        async change => applications.completion(await settingsBridge.save({ extensionId: extension.id, ...change })),
        _notify
    );
    if (application?.result.status === "running")
    {
        state.resumeConfiguration(application.key, applications.completion(application));
    }
    else if (application?.result.status === "completed")
    {
        state.observeConfiguration({ ...configuration, ...application.result });
    }
    states = { ...states, [extension.id]: state };
}

async function _setEnabled(id: string, enabled: boolean): Promise<void>
{
    toggling.set(id, Date.now());
    try
    {
        await settingsBridge.setEnabled(id, enabled);
    }
    catch (error)
    {
        _notify(settingsFailure(error));
    }
    finally
    {
        toggling.delete(id);
    }
}

function _notify(error: SettingsError): void
{
    notification = error;
}

function _closed(): void
{
    // Retain pending writes and failed edits across native closes; reloading can overwrite them.
    selectedState?.commitEdits();
    selection = "general";
    navigationRevision++;
    revealed = null;
    highlightedTarget = null;
    search.update("");
    void _ready();
}

function _query(query: string): void
{
    revealed = null;
    highlightedTarget = null;
    search.update(query);
}

async function _navigate(page: string, target: SettingsSearchTarget): Promise<void>
{
    selectedState?.commitEdits();
    if (selection !== page)
    {
        highlightedTarget = null;
        revealed = null;
    }
    selection = page;
    const revision = ++navigationRevision;
    if (page === "general")
    {
        void _loadStartup();
    }
    await tick();
    if (revision !== navigationRevision)
    {
        return;
    }
    const anchor = settingsAnchor(page, target);
    const element = document.getElementById(anchor);
    if (!element || !detailViewport?.contains(element))
    {
        highlightedTarget = null;
        search.refresh();
        _notify({ code: "unknownSetting", path: [] });
        return;
    }
    if (target.kind === "page")
    {
        detailViewport.scrollTop = 0;
    }
    else
    {
        revealSetting(detailViewport, element);
    }
    highlightedTarget = null;
    if (target.kind !== "page")
    {
        highlightedTarget = element.matches(".settings-search-target")
            ? element
            : element.querySelector<HTMLElement>(".settings-search-target");
    }
    revealed = anchor;
}

function _focusSearch(): void
{
    searchInput?.focus({ preventScroll: true });
    searchInput?.select();
}

function _searchKeydown(event: KeyboardEvent): void
{
    const modifier = customControls ? event.ctrlKey && !event.metaKey : event.metaKey && !event.ctrlKey;
    if (
        event.defaultPrevented || isCompositionKey(event) || event.altKey || event.shiftKey || !modifier
        || event.key.toLowerCase() !== "f"
    )
    {
        return;
    }
    event.preventDefault();
    if (!event.repeat)
    {
        _focusSearch();
    }
}

function _windowAction(action: SettingsWindowAction): void
{
    windowError = null;
    void settingsBridge.windowAction(action).catch(error =>
    {
        const code = {
            drag: "windowDragFailed",
            minimize: "windowMinimizeFailed",
            toggleMaximize: "windowResizeFailed",
            close: "windowCloseFailed"
        } as const;
        windowError = operationFailure(error, code[action.kind]);
    });
}
</script>

<svelte:window onkeydown={_searchKeydown} />

<div class="settings-window" class:maximized class:native-controls={!customControls}>
    <SettingsToast
        message={notification}
        onDismiss={() =>
        {
            notification = null;
        }}
    />
    {#if customControls}
        <SettingsTitleBar {maximized} onAction={_windowAction} />
    {:else}
        <!-- Reserve the native caption row so its drag region cannot cover error actions. -->
        <div class="native-titlebar">
            <div
                class="native-drag-region"
                role="presentation"
                onmousedown={event =>
                {
                    if (event.button === 0 && event.detail === 1)
                    {
                        _windowAction({ kind: "drag" });
                    }
                }}
            >
            </div>
        </div>
    {/if}
    {#if windowError}
        <div class="window-error" role="alert">
            <span>{operationErrorText(windowError, localization.locale, value => localization.text(value))}</span>
            {#if windowError.kind === "host" && windowError.code === "settingsReadyFailed"}
                <Button
                    onclick={() =>
                    {
                        void _ready();
                    }}
                >{localization.text({ message: "action-retry" })}</Button>
            {/if}
        </div>
    {/if}
    {#if presentationError}
        <div class="window-error" role="alert">
            <span>{operationErrorText(presentationError, localization.locale, value => localization.text(value))}</span>
            <Button
                disabled={preparingPresentation}
                onclick={() =>
                {
                    void _mergeLifecycle();
                }}
            >
                {localization.text({ message: "action-retry" })}
            </Button>
        </div>
    {/if}
    <main class="settings">
        <aside class="sidebar" aria-label={localization.text({ message: "settings-navigation" })}>
            <SettingsSidebar
                {extensions}
                text={_text}
                ready={snapshot !== null && !loadError}
                {selection}
                {revealed}
                {search}
                bind:input={searchInput}
                {searchShortcut}
                onQuery={_query}
                onNavigate={(page, target) =>
                {
                    void _navigate(page, target);
                }}
            />
        </aside>
        <ScrollArea class="settings-detail" bind:viewport={detailViewport}><section
                class="content"
                bind:this={detailContent}
                aria-label={localization.text({ message: "settings-content" })}
                aria-busy={loading}
            >
                {#if loading}
                    <div class="empty-state" role="status">{localization.text({ message: "settings-loading" })}</div>
                {:else if loadError}
                    <div class="empty-state" role="alert">
                        <p>{operationErrorText(loadError, localization.locale, value => localization.text(value))}</p>
                        <Button
                            onclick={() =>
                            {
                                void _load();
                            }}
                        >
                            {localization.text({ message: "action-retry" })}
                        </Button>
                    </div>
                {:else if selection === "general" && host}
                    <GeneralSettings
                        settings={host}
                        sections={snapshot?.generalSections ?? []}
                        {startup}
                        onStartupRetry={() =>
                        {
                            void _loadStartup();
                        }}
                    />
                {:else if selection === "about" && snapshot}
                    <AboutSettings version={snapshot.version} />
                {:else if selected}
                    <div class="extension-page" id={settingsAnchor(selected.id, { kind: "page" })}>
                        <header>
                            <div class="extension-heading">
                                <span class="heading-icon"><ExtensionIcon src={selected.iconUrl} /></span><div>
                                    <h1>{_text(selected.name, selected.resourceScope)}</h1>
                                    {#key selected.id}<ExtensionStatus lifecycleState={selected.state} />{/key}
                                </div>
                            </div>
                        </header>
                        {#if selected.lifecycleError}
                            <p class="lifecycle-error" role="alert">
                                {
                                    lifecycleErrorText(selected.lifecycleError, localization.locale, value => localization.text(value))
                                }
                            </p>
                        {/if}
                        {#key selected.id}
                            <form
                                onsubmit={event =>
                                {
                                    event.preventDefault();
                                    selectedState?.commitEdits();
                                }}
                            >
                                <div class="fields">
                                    <section
                                        class="property scalar settings-search-target"
                                        id={settingsAnchor(selected.id, { kind: "enabled" })}
                                        aria-labelledby="extension-enabled-title"
                                    >
                                        <div class="property-copy">
                                            <h2 id="extension-enabled-title">
                                                {localization.text({ message: "settings-enable-extension-title" })}
                                            </h2>
                                        </div>
                                        <div class="property-control">
                                            <SettingsField
                                                busy={selected.pending || toggling.has(selected.id)}
                                                label={localization.text({
                                                    message: "settings-enable-extension",
                                                    args: { name: _text(selected.name, selected.resourceScope) }
                                                })}
                                                startedAt={toggling.get(selected.id)}
                                            >
                                                <Switch
                                                    checked={selected.enabled}
                                                    label={localization.text({
                                                        message: "settings-enable-extension",
                                                        args: { name: _text(selected.name, selected.resourceScope) }
                                                    })}
                                                    onChange={enabled =>
                                                    {
                                                        if (selected)
                                                        {
                                                            void _setEnabled(selected.id, enabled);
                                                        }
                                                    }}
                                                />
                                            </SettingsField>
                                        </div>
                                    </section>
                                </div>
                                {#if !selected.configurationError && groups.length > 0}
                                    {#each groups as [group, properties] (group.id)}
                                        <section
                                            class="configuration-group"
                                            aria-label={_text(group.title, selected.resourceScope)}
                                        >
                                            <h2>{_text(group.title, selected.resourceScope)}</h2>
                                            <div class="fields">
                                                {#each properties as [key, property] (key)}
                                                    <section
                                                        class="property settings-search-target"
                                                        id={settingsAnchor(selected.id, { kind: "field", key })}
                                                        class:directory={property.type === "array" && property.items?.format === "directory"}
                                                        class:scalar={property.type === "boolean" || property.type === "integer" || property.type === "integerLimit"}
                                                        aria-labelledby={`title-${key}`}
                                                    >
                                                        {#if !(property.type === "array" && property.items?.format === "directory")}<div class="property-copy">
                                                                <h3 id={`title-${key}`}>
                                                                    {_text(property.title, selected.resourceScope)}
                                                                </h3>
                                                                {#if property.description}<p>
                                                                        {
                                                                            property.description ? _text(property.description, selected.resourceScope) : null
                                                                        }
                                                                    </p>{/if}
                                                            </div>{/if}
                                                        <div class="property-control">
                                                            <SettingsField
                                                                busy={selectedState?.phase.get(key) !== undefined}
                                                                label={_text(property.title, selected.resourceScope)}
                                                                startedAt={selectedState?.startedAt.get(key)}
                                                                progress={selectedState?.progress.get(key)}
                                                                onCommit={() =>
                                                                {
                                                                    return selectedState?.commit(key);
                                                                }}
                                                            >
                                                                {#if property.type === "array" && property.items?.format === "directory"}
                                                                    <DirectoryList
                                                                        paths={values[key] as string[]}
                                                                        maximum={property.maxItems ?? 0}
                                                                        label={_text(property.title, selected.resourceScope)}
                                                                        titleId={`title-${key}`}
                                                                        description={property.description ? _text(property.description, selected.resourceScope) : null}
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
                                                                        label={_text(property.title, selected.resourceScope)}
                                                                        id={`setting-${key}`}
                                                                        onChange={value => selectedState?.edit(key, value)}
                                                                        onCommit={() =>
                                                                        {
                                                                            return selectedState?.commit(key);
                                                                        }}
                                                                    />
                                                                {/if}
                                                            </SettingsField>
                                                        </div>
                                                    </section>
                                                {/each}
                                            </div>
                                        </section>
                                    {/each}
                                {/if}
                                {#if selected.configuration?.contribution.actions.length}
                                    {#key selected.instanceId}
                                        <SettingsActions
                                            extensionId={selected.id}
                                            iconUrl={selected.iconUrl}
                                            actions={selected.configuration.contribution.actions}
                                            presentation={presentations.get(selected.id)!}
                                            disabled={!selected.enabled || selected.pending || selected.instanceId === null}
                                            onInvoke={(actionId, confirmed) =>
                                            settingsBridge.invokeAction({
                                                extensionId: selected!.id,
                                                instanceId: selected!.instanceId!,
                                                actionId,
                                                confirmed
                                            })}
                                            onError={_notify}
                                        />
                                    {/key}
                                {/if}
                                {#if selected.configurationError}
                                    <p class="lifecycle-error" role="alert">
                                        {
                                            operationErrorText(selected.configurationError, localization.locale, value => localization.text(value))
                                        }
                                    </p>
                                {/if}
                            </form>
                        {/key}
                    </div>
                {/if}
                <SelectionHighlight container={detailContent} target={highlightedTarget} />
            </section></ScrollArea>
    </main>
</div>

<style>
.lifecycle-error {
    max-height: 35vh;
    overflow: auto;
    overflow-anchor: none;
    overflow-wrap: anywhere;
    overscroll-behavior: contain;
    white-space: pre-wrap;
}

.configuration-group > h2 { margin: 0 0 var(--space-3); font-size: var(--font-meta); color: var(--text-secondary); font-weight: 600; }
.settings-window { --settings-sidebar-width: 13rem; position: relative; display: flex; flex-direction: column; width: 100%; height: 100%; overflow: clip; border: 1px solid var(--border-subtle); border-radius: var(--radius-window); background: var(--surface-window); }
.settings-window.maximized { border: 0; border-radius: 0; }
/* macOS owns the outer clipping and overlays native controls on these surfaces. */
.settings-window.native-controls { --settings-titlebar-height: 48px; border: 0; border-radius: 0; }
.native-titlebar { position: relative; flex-shrink: 0; height: var(--settings-titlebar-height); background: linear-gradient(to right, var(--settings-sidebar-surface) var(--settings-sidebar-width), var(--surface-window) var(--settings-sidebar-width)); }
.native-drag-region { position: absolute; z-index: 1; top: 0; right: 0; left: 96px; height: var(--settings-titlebar-height); cursor: default; -webkit-user-select: none; user-select: none; }
.native-controls .sidebar { padding-top: 0; }
.window-error { flex-shrink: 0; max-height: 30vh; overflow: auto; white-space: pre-wrap; overflow-wrap: anywhere; padding: var(--space-2) var(--space-5); color: var(--text-danger); font-size: var(--font-control); }

.settings { display: grid; min-height: 0; flex: 1; grid-template-columns: var(--settings-sidebar-width) minmax(0, 1fr); width: 100%; height: 100%; background: var(--surface-window); color: var(--text-primary); font-size: var(--font-meta); }
.sidebar { display: flex; flex-direction: column; min-height: 0; gap: var(--space-4); padding: 20px 10px 12px; border-right: 1px solid var(--border-subtle); background: var(--settings-sidebar-surface); }
.extension-heading { display: flex; align-items: center; gap: 12px; }
.heading-icon { --icon-size: 48px; display: block; flex-shrink: 0; }
.content { position: relative; isolation: isolate; min-width: 0; min-height: 100%; }
.extension-page { max-width: 52rem; margin: 0 auto; padding: var(--space-4) var(--space-6) 0; }
.extension-page { display: flex; flex-direction: column; min-height: 100%; overflow-wrap: anywhere; }
header { margin-bottom: var(--settings-group-gap); }
h1 { margin: 0; font-size: 20px; line-height: 26px; font-weight: var(--settings-heading-weight); }
p { color: var(--text-secondary); font-size: var(--settings-description-size); line-height: var(--settings-description-line-height); }
form { display: flex; flex: 1; min-height: 0; flex-direction: column; }
.empty-state p { max-width: 100%; white-space: pre-wrap; overflow-wrap: anywhere; }
.empty-state { display: grid; min-height: 100%; place-content: center; justify-items: center; gap: 12px; padding: 24px; text-align: center; }
@media (width < 800px) { .settings-window { --settings-sidebar-width: 12rem; } .extension-page { padding: var(--space-4) var(--space-4) 0; } }
</style>
