<script lang="ts">
import { isCompositionKey } from "./ui/keyboard";
import { confirmAction, confirmationPending } from "./ui/confirmation";
import { configurationMenuOperation, configurationMenuSections } from "./logic/configuration-menu";
import type { ConfigurationMenu } from "./generated/ConfigurationMenu";
import ExtensionMenuIdentity from "./components/extensions/ExtensionMenuIdentity.svelte";
import Button from "./components/ui/Button.svelte";
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

let application = $state<ApplicationSnapshot | null>(null);
let failed = $state(false);
let queryFailure = $state<string | null>(null);
let invoking = $state(false);
let contextMenu = $state.raw<ContextMenuPresentation | null>(null);
let menuRequest = 0;
let menuOpen = $state(false);
let configurationPending = $state(false);
let operationFailure = $state<string | null>(null);
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
let viewInputError = $state<string | null>(null);
let viewOperation = 0;
const submitViewEvent = orderedViewEvents(tauriBridge.viewEvent, tauriBridge.invokeContextMenu);
const viewInput = viewInputScheduler();
const navigationError = $derived(navigation.error);
const rootSearchState = new RootSearchState({
    navigation: { revision: 0, current: null, busy: false, error: null, dismissCount: 0 },
    sessionId: 0,
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
            application = initial;
            const pending = pendingRootSearch;
            pendingRootSearch = null;
            if (pending)
            {
                updateRootSearch(pending);
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

function fail(error: unknown): void
{
    if (disposed)
    {
        return;
    }
    failed = true;
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
        if (token !== menuRequest || !document.hasFocus() || actions.length === 0)
        {
            return;
        }
        contextMenu = { request, actions, position };
        menuOpen = true;
    }
    catch (error)
    {
        if (token === menuRequest)
        {
            operationFailure = String(error);
        }
    }
}

function closeContextMenu(): void
{
    menuRequest++;
    menuOpen = false;
}

async function invokeContextMenu(actionId: string): Promise<void>
{
    const current = contextMenu;
    closeContextMenu();
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
                title: action.confirmation_title,
                destructive: action.style === "destructive",
                label: action.title,
                message: action.confirmation_description ?? "",
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
        operationFailure = String(error);
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
        operationFailure = String(error);
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
            operationFailure = String(error);
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
        title: "Settings",
        enabled: true,
        allow_default_execution: false,
        style: "secondary" as const,
        group: null
    };
    contextMenu = {
        request: null,
        settingsExtensionId: extension?.extensionId,
        extensionIconUrl: extension?.extensionIconUrl,
        heading: extension?.extensionName ?? "Nanika",
        shortcuts: { settings: [application?.platform === "macos" ? "⌘" : "Ctrl", ","] },
        actions: [settings],
        ...(request && snapshot
            ? {
                configurationMenu: { request, snapshot },
                sections: [...configurationMenuSections(snapshot, configurationPending), { items: [settings] }]
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
        queryFailure = `Search supports up to ${application.maxQueryChars} characters. Edit the input to continue.`;
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
            queryFailure = "Search could not be updated. Try again.";
            if (import.meta.env.DEV)
            {
                observeSearch(requestId, "rejected");
            }
        }
    }
}

async function invokeCandidate(result: SearchResult, confirmed: boolean): Promise<void>
{
    if (
        invoking || navigation.busy || !application || rootSearch.requestId !== latestRequestId
        || rootSearch.phase !== "ready" || (confirmed ? !result.confirmationTitle : !result.allowDefaultExecution)
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
            actionId: result.actionId,
            confirmed
        };
        if (
            confirmed
            && !await confirmAction({
                destructive: result.actionStyle === "destructive",
                title: result.confirmationTitle!,
                label: result.title,
                message: result.confirmationDescription ?? "",
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
        operationFailure = String(error);
    }
    finally
    {
        invoking = false;
    }
}

function updateRootSearch(next: RootSearchSnapshot): void
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
    lastRevision = next.revision;
    updateNavigation(next.navigation);
    if (next.requestId !== latestRequestId)
    {
        return;
    }
    if (next.phase === "error")
    {
        fail(next.error ?? "Search failed without an error message.");
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
            operationFailure = String(error);
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
</script>

<svelte:window onkeydowncapture={controlLauncherKeyboard} />

{#if failed}
    <main class="fatal" role="alert">
        <strong>Search is unavailable.</strong>
        <Button onclick={() => window.location.reload()}>Try again</Button>
    </main>
{:else}
    <svelte:boundary onerror={(error => fail(error))}>
        {#if navigation.current}
            {#key navigation.current.routeId}
                <ExtensionView
                    canLoadMore={viewCanLoadMore}
                    snapshot={navigation.current}
                    resourceOrigin={application?.resourceOrigin ?? ""}
                    query={viewQuery}
                    busy={viewPending || viewInputError !== null}
                    error={viewInputError ?? operationFailure ?? navigationError}
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
                platform={application?.platform ?? null}
                searchState={rootSearchState}
                {hasCompletedSearch}
                appMenuOpen={menuOpen && contextMenu?.request === null}
                onAppMenu={trigger =>
                {
                    void showAppMenu(trigger);
                }}
                inputError={queryFailure}
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
        {#if (operationFailure || navigationError) && !navigation.current}
            <div class="operation-failure" role="alert">{operationFailure ?? navigationError}</div>
        {/if}
        {#snippet failed()}
            <main class="fatal" role="alert">Search could not be displayed.</main>
        {/snippet}
    </svelte:boundary>
{/if}

<ContextMenu
    actions={contextMenu?.actions ?? []}
    sections={contextMenu?.sections}
    position={contextMenu?.position ?? null}
    anchor={contextMenu?.anchor}
    heading={contextMenu?.heading}
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
            <ExtensionMenuIdentity name={contextMenu.heading!} iconUrl={contextMenu.extensionIconUrl!} />
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
