<script lang="ts">
import {
    operationErrorText as formatOperationError,
    operationFailure as retainOperationFailure
} from "./localization/operation-errors";
import type { SettingsError } from "./settings/SettingsError";
import { settingsErrorText, settingsFailure } from "./settings/error-presentation";
import { prepareRootPresentation } from "./localization/root-presentation";
import { ExtensionPresentation } from "./localization/ExtensionPresentation";
import { ResourceCache } from "./localization/ResourceCache";
import { TextPresentation } from "./localization/TextPresentation";
import { actionFields } from "./localization/action-fields";
import { viewFields } from "./localization/view-fields";
import type { LocalizedText } from "./generated/LocalizedText";
import type { LocalizedMessage } from "./generated/LocalizedMessage";
import type { RootPresentation } from "./types/RootPresentation";
import { localization } from "./localization/state.svelte";
import { isCompositionKey } from "./ui/keyboard";
import { confirmAction, confirmationPending } from "./ui/confirmation";
import { configurationMenuOperation, configurationMenuSections } from "./logic/configuration-menu";
import type { ConfigurationMenu } from "./generated/ConfigurationMenu";
import ExtensionMenuIdentity from "./components/extensions/ExtensionMenuIdentity.svelte";
import Button from "./components/ui/Button.svelte";
import LauncherFailure from "./components/launcher/LauncherFailure.svelte";
import type { OperationError } from "./generated/OperationError";
import { resolveActionShortcut } from "./logic/action-shortcuts";
import { RootSearchState } from "./logic/RootSearchState.svelte";
import { onMount, tick } from "svelte";

import { uiActivity } from "./ui/activity";
import { tauriBridge } from "./bridge";
import { orderedViewEvents } from "./bridge/orderedViewEvents";
import { viewInputScheduler } from "./bridge/viewInputScheduler";
import ContextMenu from "./components/ui/ContextMenu.svelte";
import AppMenuBrand from "./components/launcher/AppMenuBrand.svelte";
import type { ExtensionViewSnapshot } from "./generated/ExtensionViewSnapshot";
import type { ContextMenuPresentation } from "./types/ContextMenuPresentation";
import type { MenuTarget } from "./generated/MenuTarget";
import { syntaxService } from "./syntax/service";
import ExtensionView from "./components/extensions/ExtensionView.svelte";
import RootSearch from "./components/launcher/RootSearch.svelte";
import type { ApplicationSnapshot } from "./generated/ApplicationSnapshot";
import type { NavigationSnapshot } from "./types/NavigationSnapshot";
import type { RootSearchSnapshot } from "./types/RootSearchSnapshot";
import type { SearchResult } from "./generated/SearchResult";
import type { ViewInteraction } from "./types/ViewInteraction";
import type { ViewEventReceipt } from "./generated/ViewEventReceipt";
import type { SearchObservation } from "./development";

$effect(() =>
{
    document.documentElement.lang = localization.locale;
});

let application = $state<ApplicationSnapshot | null>(null);
const resources = new ResourceCache(tauriBridge.readResources);
let viewPresentation = $state.raw<ExtensionPresentation | null>(null);
let viewTexts = $state.raw<TextPresentation | null>(null);
let rootPresentations = $state.raw<ReadonlyMap<string, RootPresentation>>(new Map());
let presentationInstance: number | null = null;
let presentationSession: number | null = null;
let presentationError = $state<OperationError | null>(null);
let preparingPresentation = $state(false);
let pendingPresentation: RootSearchSnapshot | null = null;
let presentationAttempt = 0;
let failure = $state<OperationError | null>(null);
let startupFailed = $state(false);
let queryFailure = $state<LocalizedMessage | OperationError | null>(null);
const queryErrorText = $derived.by(() =>
{
    if (!queryFailure)
    {
        return null;
    }
    return "kind" in queryFailure
        ? formatOperationError(queryFailure, localization.locale, value => localization.text(value))
        : localization.text(queryFailure);
});
let invoking = $state(false);
let contextMenu = $state.raw<ContextMenuPresentation | null>(null);
let menuRequest = 0;
let menuOpen = $state(false);
const menuHeading = $derived(
    menuOpen && contextMenu?.headingPresentation
        ? contextMenu.headingPresentation.name(localization.locale)
        : contextMenu?.heading
);
const menuSections = $derived(
    menuOpen && contextMenu?.localizedSections
        ? contextMenu.localizedSections[localization.locale]
        : contextMenu?.sections
);
let configurationPending = $state(false);
let operationFailure = $state<SettingsError | null>(null);
// A completed empty view survives pending queries just like a completed list.
let hasCompletedSearch = $state(false);
let navigation = $state.raw<NavigationSnapshot>({
    revision: 0,
    current: null,
    busy: false,
    error: null,
    dismissCount: 0
});
let viewPending = $state(false);
let viewCanLoadMore = $state(false);
let viewQuery = $state("");
let viewInputError = $state<LocalizedMessage | null>(null);
let viewOperation = 0;
const submitViewEvent = orderedViewEvents(tauriBridge.viewEvent, tauriBridge.invokeContextMenu);
const viewInput = viewInputScheduler();
const navigationError = $derived(
    navigation.error
        ? formatOperationError(
            retainOperationFailure(navigation.error, "invocationFailed"),
            localization.locale,
            value => localization.text(value)
        )
        : null
);
const operationErrorText = $derived(
    operationFailure === null
        ? null
        : settingsErrorText(operationFailure, localization.locale, value => localization.text(value))
);
const rootSearchState = new RootSearchState({
    navigation: { revision: 0, current: null, busy: false, error: null, dismissCount: 0 },
    sessionId: 0,
    localization: { preference: "system", locale: "en", revision: 0 },
    requestId: 0,
    revision: 0,
    query: "",
    results: [],
    resultRevision: 0,
    resultOffset: 0,
    totalResults: 0,
    phase: "searching",
    error: null,
    textInvalidations: [],
    pendingExtensions: [],
    warnings: []
});
const rootSearch = $derived(rootSearchState.snapshot);
let latestRequestId = $state(0);
let desiredQuery = "";
let lastRevision = 0;
let pendingRootSearch: RootSearchSnapshot | null = null;
let disposed = false;

onMount(() =>
{
    const stopActivity = uiActivity.subscribe(activity =>
    {
        if (!activity.visible || !activity.focused)
        {
            closeContextMenu();
            contextMenu = null;
        }
    });
    void (async () =>
    {
        try
        {
            const initial = await tauriBridge.openSession(updateRootSearch, fail);
            if (disposed)
            {
                await tauriBridge.closeSession(initial.sessionId);
                return;
            }
            localization.initialize(initial.sessionId, initial.localization, initial.platform);
            application = initial;
            const pending = pendingRootSearch;
            pendingRootSearch = null;
            if (pending)
            {
                await updateRootSearch(pending);
            }
            if (latestRequestId > 0)
            {
                await submitQuery(latestRequestId, desiredQuery);
            }
        }
        catch (error)
        {
            fail(error);
        }
    })();
    return () =>
    {
        stopActivity();
        disposed = true;
        if (application)
        {
            syntaxService.suspend();
            syntaxService.cache.clear();
            void tauriBridge.closeSession(application.sessionId).catch(console.error);
        }
    };
});

function fail(error: unknown, startup = false): void
{
    if (disposed)
    {
        return;
    }
    failure = retainOperationFailure(error, startup ? "runtimeStartupFailed" : "runtimeUnavailable");
    startupFailed = startup;
    if (import.meta.env.DEV)
    {
        observeSearch(latestRequestId, "failed");
    }
    console.error("Search failed", error);
}

async function showContextMenu(target: MenuTarget, position: [number, number] | null): Promise<void>
{
    closeContextMenu();
    if (!application)
    {
        return;
    }
    const token = ++menuRequest;
    const request = { sessionId: application.sessionId, target };
    operationFailure = null;
    try
    {
        const actions = await tauriBridge.readContextMenu(request);
        const keyboardLayout = target.kind === "search" && actions.some(action =>
                action.shortcut?.key.kind === "physical"
            )
            ? await tauriBridge.readKeyboardLayout()
            : {};
        if (token !== menuRequest || !document.hasFocus() || actions.length === 0)
        {
            return;
        }
        const bundle = target.kind === "view"
            ? viewPresentation?.resources
            : rootPresentations.get(target.extensionId)?.presentation.resources;
        const presentation = new TextPresentation(bundle, application.platform, actionFields(actions));
        const platform = application.platform;
        const shortcuts = target.kind === "search"
            ? Object.fromEntries(actions.flatMap(action =>
                action.shortcut
                    ? [[action.id, resolveActionShortcut(action.shortcut, platform, keyboardLayout).keys]]
                    : []
            ))
            : {};
        contextMenu = { request, actions, position, presentation, shortcuts };
        menuOpen = true;
    }
    catch (error)
    {
        if (token === menuRequest)
        {
            operationFailure = settingsFailure(error);
        }
    }
}

function closeContextMenu(): void
{
    menuRequest++;
    if (menuOpen && contextMenu)
    {
        contextMenu = {
            ...contextMenu,
            heading: contextMenu.headingPresentation?.name(localization.locale) ?? contextMenu.heading,
            sections: menuSections,
            locale: localization.locale
        };
    }
    menuOpen = false;
}

async function invokeContextMenu(actionId: string): Promise<void>
{
    closeContextMenu();
    const current = contextMenu;
    if (!current)
    {
        return;
    }
    try
    {
        const action = (current.sections?.flatMap(section => section.items) ?? current.actions)
            .find(item => item.id === actionId);
        let confirmed = false;
        if (action?.confirmation_title)
        {
            const fallbackIcon = current.extensionIconUrl
                ? { kind: "image" as const, url: current.extensionIconUrl }
                : null;
            confirmed = await confirmAction({
                content: () => ({
                    title: current.presentation!.text(action.confirmation_title!, localization.locale),
                    label: current.presentation!.text(action.title, localization.locale),
                    message: action.confirmation_description
                        ? current.presentation!.text(action.confirmation_description, localization.locale)
                        : ""
                }),
                destructive: action.style === "destructive",
                icon: action.icon
                    ? { kind: "symbol", name: action.icon }
                    : fallbackIcon
            });
            if (!confirmed || disposed)
            {
                return;
            }
        }
        if (current.request)
        {
            const request = current.request;
            if (request.target.kind === "view")
            {
                if (viewInput.busy)
                {
                    return;
                }
                await runViewInput(
                    {
                        kind: "actionInvoked",
                        item_id: request.target.itemId,
                        action_id: actionId,
                        invocation: confirmed ? "confirmed" : "explicit"
                    },
                    () => submitViewEvent.menu(request, actionId, confirmed)
                );
            }
            else
            {
                if (invoking || navigation.busy)
                {
                    return;
                }
                invoking = true;
                operationFailure = null;
                try
                {
                    await tauriBridge.invokeContextMenu(request, actionId, confirmed);
                }
                finally
                {
                    invoking = false;
                }
            }
        }
        else if (current.configurationMenu && actionId !== "settings")
        {
            if (configurationPending)
            {
                return;
            }
            const operation = configurationMenuOperation(current.configurationMenu.snapshot, actionId, confirmed);
            if (!operation)
            {
                return;
            }
            operationFailure = null;
            configurationPending = true;
            try
            {
                await tauriBridge.invokeConfigurationMenu(current.configurationMenu.request, operation);
            }
            catch (error)
            {
                operationFailure = settingsFailure(error);
            }
            finally
            {
                configurationPending = false;
                // A menu reopened during this write must fetch the committed state next time.
                if (contextMenu?.configurationMenu?.request.routeId === current.configurationMenu.request.routeId)
                {
                    closeContextMenu();
                }
            }
        }
        else if (actionId === "settings")
        {
            await openSettings(current.settingsExtensionId);
        }
    }
    catch (error)
    {
        operationFailure = settingsFailure(error);
    }
}

async function openSettings(extensionId?: string): Promise<void>
{
    closeContextMenu();
    operationFailure = null;
    try
    {
        await tauriBridge.openSettings(extensionId);
    }
    catch (error)
    {
        operationFailure = settingsFailure(error);
    }
}

async function showAppMenu(trigger: HTMLButtonElement, extension?: ExtensionViewSnapshot): Promise<void>
{
    const wasOpen = menuOpen && contextMenu?.request === null
        && contextMenu.settingsExtensionId === extension?.extensionId;
    closeContextMenu();
    if (wasOpen)
    {
        return;
    }
    const token = menuRequest;
    const request = extension && application
        ? { sessionId: application.sessionId, routeId: extension.routeId, instanceId: extension.instanceId }
        : null;
    let snapshot: ConfigurationMenu | null = null;
    operationFailure = null;
    try
    {
        if (request)
        {
            snapshot = await tauriBridge.readConfigurationMenu(request);
        }
    }
    catch (error)
    {
        if (token === menuRequest)
        {
            operationFailure = settingsFailure(error);
        }
        return;
    }
    await tick();
    if (token !== menuRequest || !document.hasFocus())
    {
        return;
    }
    if (
        request
        && (navigation.current?.routeId !== request.routeId || navigation.current.instanceId !== request.instanceId)
    )
    {
        return;
    }
    const settings = {
        id: "settings",
        title: localization.text({ message: "settings-title" }),
        enabled: true,
        allow_default_execution: false,
        style: "secondary" as const,
        group: null
    };
    const settingsSections: NonNullable<ContextMenuPresentation["localizedSections"]> = {
        en: [{ items: [{ ...settings, title: localization.text({ message: "settings-title" }, undefined, "en") }] }],
        "zh-Hans": [{
            items: [{ ...settings, title: localization.text({ message: "settings-title" }, undefined, "zh-Hans") }]
        }]
    };
    let localizedSections = settingsSections;
    let actionPresentation: TextPresentation | undefined;
    try
    {
        if (request && snapshot && viewPresentation)
        {
            const prepared = viewPresentation;
            actionPresentation = new TextPresentation(
                prepared.resources,
                application!.platform,
                actionFields(
                    snapshot.groups.flatMap(group =>
                        group.items.flatMap(item => item.kind === "action" ? [item.action] : [])
                    )
                )
            );
            localizedSections = {
                en: [
                    ...configurationMenuSections(snapshot, configurationPending, value => prepared.text(value, "en")),
                    ...settingsSections.en
                ],
                "zh-Hans": [
                    ...configurationMenuSections(
                        snapshot,
                        configurationPending,
                        value => prepared.text(value, "zh-Hans")
                    ),
                    ...settingsSections["zh-Hans"]
                ]
            };
        }
    }
    catch (error)
    {
        operationFailure = settingsFailure(error);
        return;
    }
    contextMenu = {
        presentation: actionPresentation,
        localizedSections,
        request: null,
        settingsExtensionId: extension?.extensionId,
        extensionIconUrl: extension?.extensionIconUrl,
        heading: extension ? viewPresentation?.name(localization.locale) : "Nanika",
        headingPresentation: extension ? viewPresentation ?? undefined : undefined,
        shortcuts: { settings: [application?.platform === "macos" ? "⌘" : "Ctrl", ","] },
        actions: [settings],
        ...(request && snapshot
            ? {
                configurationMenu: { request, snapshot },
                sections: localizedSections?.[localization.locale]
            }
            : {}),
        position: null,
        anchor: trigger
    };
    menuOpen = true;
}

async function publishQuery(query: string): Promise<void>
{
    operationFailure = null;
    const requestId = ++latestRequestId;
    if (import.meta.env.DEV)
    {
        observeSearch(requestId, "input");
    }
    desiredQuery = query;
    queryFailure = null;
    await submitQuery(requestId, query);
}

async function submitQuery(requestId: number, query: string): Promise<void>
{
    if (!application)
    {
        return;
    }
    if ([...query].length > application.maxQueryChars)
    {
        queryFailure = { message: "search-input-too-long", args: { maximum: application.maxQueryChars } };
        if (import.meta.env.DEV)
        {
            observeSearch(requestId, "rejected");
        }
        return;
    }
    try
    {
        // RPC only acknowledges submission; search state arrives independently through the Channel.
        await tauriBridge.publishQuery({ sessionId: application.sessionId, requestId, query });
    }
    catch (error)
    {
        if (requestId === latestRequestId)
        {
            console.error("Search query could not be submitted", error);
            queryFailure = retainOperationFailure(error, "searchSubmissionFailed");
            if (import.meta.env.DEV)
            {
                observeSearch(requestId, "rejected");
            }
        }
    }
}

async function invokeCandidate(
    result: SearchResult,
    actionId: string,
    invocation: "default" | "explicit"
): Promise<void>
{
    const action = result.actions.find(item => item.id === actionId && item.enabled);
    if (
        !action || !rootSearch.results.includes(result) || menuOpen || confirmationPending()
        || invoking || navigation.busy || !application || rootSearch.requestId !== latestRequestId
        || rootSearch.phase !== "ready"
        || (invocation === "default"
            && (actionId !== result.actionId || (!action.confirmation_title && !action.allow_default_execution)))
    )
    {
        return;
    }
    try
    {
        invoking = true;
        operationFailure = null;
        const request = {
            sessionId: application.sessionId,
            requestId: rootSearch.requestId,
            resultRevision: rootSearch.resultRevision,
            extensionId: result.extensionId,
            entryId: result.entryId,
            actionId,
            invocation: action.confirmation_title ? "confirmed" as const : invocation
        };
        const prepared = rootPresentations.get(result.extensionId)?.presentation;
        if (
            action.confirmation_title
            && !await confirmAction({
                destructive: action.style === "destructive",
                content: () => ({
                    title: prepared!.text(action.confirmation_title!, localization.locale),
                    label: prepared!.text(action.title, localization.locale),
                    message: action.confirmation_description
                        ? prepared!.text(action.confirmation_description, localization.locale)
                        : ""
                }),
                icon: result.icon
            })
        )
        {
            return;
        }
        if (
            disposed || rootSearch.phase !== "ready" || rootSearch.requestId !== request.requestId
            || rootSearch.resultRevision !== request.resultRevision || application?.sessionId !== request.sessionId
        )
        {
            return;
        }
        await tauriBridge.invokeCandidate(request);
    }
    catch (error)
    {
        console.error("Action failed", error);
        operationFailure = retainOperationFailure(error, "invocationUnconfirmed");
    }
    finally
    {
        invoking = false;
    }
}

async function updateRootSearch(next: RootSearchSnapshot): Promise<void>
{
    if (disposed)
    {
        return;
    }
    if (!application)
    {
        pendingRootSearch = next;
        return;
    }
    if (
        next.sessionId !== application.sessionId
        || next.revision <= lastRevision
    )
    {
        return;
    }
    for (const notice of next.textInvalidations)
    {
        const scope = JSON.stringify([next.sessionId, notice.extensionId, notice.instanceId]);
        if (notice.text.documents === null)
        {
            syntaxService.invalidate(scope);
        }
        else
        {
            for (const id of notice.text.documents)
            {
                syntaxService.invalidate(scope, id);
            }
        }
    }
    const localizationChanged = localization.accept(next.sessionId, next.localization);
    const pending = pendingPresentation;
    if (
        localizationChanged && presentationError && pending
        && next.requestId === pending.requestId && next.resultRevision === pending.resultRevision
        && next.resultOffset === pending.resultOffset && next.totalResults === pending.totalResults
        && next.results === pending.results && next.phase === pending.phase
        && next.navigation.revision === pending.navigation.revision
    )
    {
        // A locale-only update must not retry failed resource work or replace the coherent UI.
        // Retain its newer transport revision for an explicit retry of the same presentation.
        pendingPresentation = next;
        return;
    }
    const current = next.navigation.revision > navigation.revision ? next.navigation.current : navigation.current;
    const session = application;
    const attempt = ++presentationAttempt;
    pendingPresentation = next;
    preparingPresentation = true;
    try
    {
        let prepared = viewPresentation;
        if (
            current && (
                presentationSession !== session.sessionId || presentationInstance !== current.instanceId
                || !prepared?.matches(
                    current.extensionId,
                    current.extensionName,
                    current.resourceScope,
                    session.platform
                )
            )
        )
        {
            const bundle = current.resourceScope
                ? await resources.load({
                    surface: "launcher",
                    sessionId: session.sessionId,
                    instanceId: current.instanceId,
                    scope: current.resourceScope
                })
                : undefined;
            prepared = new ExtensionPresentation(
                current.extensionId,
                current.extensionName,
                current.resourceScope,
                bundle,
                session.platform
            );
        }
        const preparedViewTexts =
            current && (current.view !== navigation.current?.view || !viewTexts || prepared !== viewPresentation)
                ? new TextPresentation(prepared?.resources, session.platform, viewFields(current.view))
                : viewTexts;
        // Searching retains the previous rows, so their resource owners must remain retained too.
        const preparedRootPresentation = next.phase === "ready" && next.requestId === latestRequestId
                && (next.results !== rootSearch.results || next.sessionId !== rootSearch.sessionId)
            ? await prepareRootPresentation(next.results, session, rootPresentations, resources)
            : rootPresentations;
        if (disposed || attempt !== presentationAttempt || application?.sessionId !== session.sessionId)
        {
            return;
        }
        viewPresentation = current ? prepared : null;
        viewTexts = current ? preparedViewTexts : null;
        rootPresentations = preparedRootPresentation;
        presentationInstance = current?.instanceId ?? null;
        presentationSession = session.sessionId;
        pendingPresentation = null;
        presentationError = null;
    }
    catch (error)
    {
        if (attempt === presentationAttempt)
        {
            presentationError = retainOperationFailure(error, "localizationReadFailed");
        }
        return;
    }
    finally
    {
        if (attempt === presentationAttempt)
        {
            preparingPresentation = false;
        }
    }
    lastRevision = next.revision;
    updateNavigation(next.navigation);
    if (next.requestId !== latestRequestId)
    {
        return;
    }
    if (next.phase === "error")
    {
        fail(next.error ?? { kind: "host", code: "runtimeStartupFailed" }, true);
        return;
    }
    // Keep the last coherent list noninteractive until Rust can validate the new results.
    if (next.phase === "ready")
    {
        hasCompletedSearch = true;
        if (import.meta.env.DEV)
        {
            observeSearch(next.requestId, "received");
        }
    }
    rootSearchState.accept(next);
    if (import.meta.env.DEV && next.phase === "ready")
    {
        void tick().then(() =>
        {
            if (!disposed && next.requestId === latestRequestId && next.revision === lastRevision)
            {
                observeSearch(next.requestId, "committed");
            }
        });
    }
}

function updateNavigation(next: NavigationSnapshot): void
{
    if (next.revision <= navigation.revision)
    {
        return;
    }
    const dismiss = next.dismissCount > navigation.dismissCount;
    if (next.error)
    {
        console.error("View operation failed", next.error);
    }
    if (
        next.current?.routeId !== navigation.current?.routeId
        || next.current?.instanceId !== navigation.current?.instanceId
    )
    {
        closeContextMenu();
    }
    navigation = next;
    viewInput.update(next);
    if (dismiss)
    {
        void tauriBridge.dismissLauncher().catch(fail);
    }
    reconcileViewInput();
}

function changeViewQuery(text: string, minimumItems: number): void
{
    viewInput.query(text, minimumItems);
    reconcileViewInput();
}

function resumeView(): void
{
    viewInput.resume();
    reconcileViewInput();
}

function reconcileViewInput(): void
{
    viewPending = viewInput.busy;
    viewCanLoadMore = viewInput.canLoadMore;
    viewInputError = viewInput.inputError;
    viewQuery = viewInput.queryText;
    if (disposed || !application)
    {
        return;
    }
    const next = viewInput.takeNext();
    if (next)
    {
        void sendViewEvent(next);
    }
}

async function sendViewEvent(event: ViewInteraction | null): Promise<number | null>
{
    const current = navigation.current;
    if (!application || !current || viewInput.busy)
    {
        return null;
    }
    const request = {
        sessionId: application.sessionId,
        routeId: current.routeId,
        revision: current.revision,
        operation: event === null ? { kind: "back" as const } : { kind: "event" as const, event }
    };
    return runViewInput(event, () => submitViewEvent.event(request));
}

async function runViewInput(
    event: ViewInteraction | null,
    submit: () => Promise<ViewEventReceipt | null>
): Promise<number | null>
{
    const operation = ++viewOperation;
    const blocking = viewInput.begin(event);
    let receipt: ViewEventReceipt | null = null;
    reconcileViewInput();
    operationFailure = null;
    try
    {
        receipt = await submit();
        return receipt?.viewRevision ?? null;
    }
    catch (error)
    {
        if (operation === viewOperation)
        {
            console.error("View operation failed", error);
            operationFailure = retainOperationFailure(error, "viewOperationUnconfirmed");
        }
        return null;
    }
    finally
    {
        viewInput.complete(blocking, receipt);
        reconcileViewInput();
    }
}

function observeSearch(requestId: number, stage: SearchObservation["stage"]): void
{
    document.dispatchEvent(
        new CustomEvent<SearchObservation>("nanika:search-observation", {
            detail: { requestId, stage, time: performance.now() }
        })
    );
}

function controlLauncherKeyboard(event: KeyboardEvent): void
{
    if (confirmationPending())
    {
        return;
    }
    const settingsModifier = application?.platform === "macos"
        ? event.metaKey && !event.ctrlKey
        : application?.platform === "windows" && event.ctrlKey && !event.metaKey;
    if (
        !isCompositionKey(event) && settingsModifier
        && event.key === "," && !event.altKey && !event.shiftKey
    )
    {
        event.preventDefault();
        event.stopPropagation();
        if (!event.repeat)
        {
            void openSettings(navigation.current?.extensionId);
        }
        return;
    }
    // Bits owns menu navigation and Escape; preventing default suppresses its handlers.
    if (event.target instanceof Element && event.target.closest("[role=menu]"))
    {
        return;
    }
    if (event.key === "Tab")
    {
        event.preventDefault();
    }
}

function _menuText(value: LocalizedText): string
{
    if (typeof value === "string")
    {
        return value;
    }
    if (!contextMenu?.presentation)
    {
        throw new Error("The menu presentation is unavailable.");
    }
    return contextMenu.presentation.text(value, menuOpen ? localization.locale : contextMenu.locale!);
}
</script>

<svelte:window onkeydowncapture={controlLauncherKeyboard} />

{#if failure}
    <LauncherFailure error={failure} startup={startupFailed} sessionId={application?.sessionId ?? null} />
{:else}
    <svelte:boundary onerror={(error => fail(error))}>
        {#if navigation.current}
            {#key navigation.current.routeId}
                <ExtensionView
                    canLoadMore={viewCanLoadMore}
                    snapshot={navigation.current}
                    extensionName={viewPresentation!.name(localization.locale)}
                    presentation={viewTexts!}
                    resourceOrigin={application?.resourceOrigin ?? ""}
                    query={viewQuery}
                    busy={viewPending || viewInputError !== null}
                    error={viewInputError ? localization.text(viewInputError) : operationErrorText ?? navigationError}
                    onQuery={changeViewQuery}
                    onEvent={sendViewEvent}
                    textScope={JSON.stringify([application?.sessionId ?? 0, navigation.current.extensionId, navigation.current.instanceId])}
                    onReadText={read => tauriBridge.readText({ sessionId: application?.sessionId ?? 0, routeId: navigation.current!.routeId, read })}
                    onResume={resumeView}
                    menuOpen={menuOpen && contextMenu?.settingsExtensionId === navigation.current.extensionId}
                    onMenu={trigger => showAppMenu(trigger, navigation.current!)}
                    onContextMenu={(itemId, position) =>
                    {
                        const route = navigation.current;
                        return route
                            ? showContextMenu({ kind: "view", routeId: route.routeId, revision: route.revision, itemId }, position)
                            : Promise.resolve();
                    }}
                    onBack={() =>
                    {
                        void sendViewEvent(null);
                    }}
                />
            {/key}
        {:else}
            <RootSearch
                presentations={rootPresentations}
                platform={application?.platform ?? null}
                searchState={rootSearchState}
                {hasCompletedSearch}
                appMenuOpen={menuOpen && contextMenu?.request === null}
                onAppMenu={trigger =>
                {
                    void showAppMenu(trigger);
                }}
                inputError={queryErrorText}
                busy={invoking || navigation.busy || !application
                || rootSearch.requestId !== latestRequestId
                || rootSearch.phase !== "ready"}
                onQuery={publishQuery}
                onRange={request =>
                {
                    void tauriBridge.readResults(request).catch(fail);
                }}
                onDismiss={() =>
                {
                    tauriBridge.dismissLauncher().catch(fail);
                }}
                onInvoke={invokeCandidate}
                onContextMenu={(result, position) =>
                showContextMenu({
                    kind: "search",
                    requestId: rootSearch.requestId,
                    resultRevision: rootSearch.resultRevision,
                    extensionId: result.extensionId,
                    entryId: result.entryId
                }, position)}
            />
        {/if}
        {#if presentationError}
            <div class="operation-failure" role="alert">
                <span>{
                    formatOperationError(presentationError, localization.locale, value => localization.text(value))
                }</span>
                <Button
                    disabled={preparingPresentation}
                    onclick={() =>
                    {
                        if (pendingPresentation)
                        {
                            void updateRootSearch(pendingPresentation);
                        }
                    }}
                >{localization.text({ message: "action-retry" })}</Button>
            </div>
        {/if}
        {#if (operationFailure || navigationError) && !navigation.current}
            <div class="operation-failure" role="alert">{operationErrorText ?? navigationError}</div>
        {/if}
        {#snippet failed()}
            <main class="fatal" role="alert">{localization.text({ message: "launcher-display-failed" })}</main>
        {/snippet}
    </svelte:boundary>
{/if}

<ContextMenu
    text={_menuText}
    actions={contextMenu?.actions ?? []}
    sections={menuSections}
    position={contextMenu?.position ?? null}
    anchor={contextMenu?.anchor}
    heading={menuHeading}
    shortcuts={contextMenu?.shortcuts}
    open={menuOpen}
    onClose={closeContextMenu}
    onClosed={() =>
    {
        if (!menuOpen)
        {
            contextMenu = null;
        }
    }}
    onInvoke={id =>
    {
        void invokeContextMenu(id);
    }}
>
    {#snippet footer()}
        {#if contextMenu?.settingsExtensionId}
            <ExtensionMenuIdentity name={menuHeading!} iconUrl={contextMenu.extensionIconUrl!} />
        {:else if application}<AppMenuBrand version={application.version} />{/if}
    {/snippet}
</ContextMenu>

<style>
.fatal {
  display: grid;
  width: 100%;
  height: 100%;
  place-content: center;
  gap: var(--space-2);
  padding: var(--space-6);
  border: 0;
  border-radius: var(--radius-window);
  background: var(--surface-window);
  color: var(--text-primary);
  text-align: center;
}

.operation-failure {
  position: absolute;
  white-space: pre-wrap;
  overflow-wrap: anywhere;
  max-height: 35%;
  overflow: auto;
  right: var(--space-4);
  bottom: var(--space-4);
  left: var(--space-4);
  padding: var(--space-3);
  border: 1px solid var(--border-window);
  border-radius: var(--radius-row);
  background: var(--surface-raised);
  color: var(--text-primary);
  font-size: var(--font-meta);
}
</style>
