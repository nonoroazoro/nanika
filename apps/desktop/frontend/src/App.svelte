<script lang="ts">
import { onMount, tick } from "svelte";

import { tauriBridge } from "./bridge";
import { orderedViewEvents } from "./bridge/orderedViewEvents";
import { viewInputScheduler } from "./bridge/viewInputScheduler";
import { ExtensionView, RootSearch } from "./components";
import type {
    ApplicationSnapshot,
    NavigationSnapshot,
    RootSearchSnapshot,
    SearchResult,
    ViewEvent,
    ViewEventReceipt
} from "./types";
import type { SearchObservation } from "./development";

let application = $state<ApplicationSnapshot | null>(null);
let failed = $state(false);
let queryFailure = $state<string | null>(null);
let invoking = $state(false);
let refreshing = $state(false);
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
let viewInputError = $state<string | null>(null);
let viewOperation = 0;
const submitViewEvent = orderedViewEvents(tauriBridge.viewEvent);
const viewInput = viewInputScheduler();
const navigationError = $derived(navigation.error ? "The action could not be completed. Try again." : null);
let rootSearch = $state.raw<RootSearchSnapshot>({
    navigation: { revision: 0, current: null, busy: false, error: null, dismissCount: 0 },
    sessionId: 0,
    requestId: 0,
    revision: 0,
    query: "",
    results: [],
    phase: "searching",
    error: null,
    warnings: []
});
let latestRequestId = $state(0);
let desiredQuery = "";
let lastRevision = 0;
let pendingRootSearch: RootSearchSnapshot | null = null;
let disposed = false;

onMount(() =>
{
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
        disposed = true;
        if (application)
        {
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
        // The RPC response acknowledges submission only. Search state comes solely
        // from the session Channel, even when it arrives before this Promise resolves.
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

async function refreshSearch(): Promise<void>
{
    if (refreshing || invoking || navigation.busy || navigation.current || !application)
    {
        return;
    }
    refreshing = true;
    operationFailure = null;
    try
    {
        await tauriBridge.refreshSearch(application.sessionId);
    }
    catch (error)
    {
        console.error("Search could not be refreshed", error);
        operationFailure = "Search could not be refreshed. Try again.";
    }
    finally
    {
        refreshing = false;
    }
}

async function invokeCandidate(result: SearchResult): Promise<void>
{
    if (
        invoking || refreshing || !application || rootSearch.requestId !== latestRequestId
        || rootSearch.phase !== "ready"
    )
    {
        return;
    }
    try
    {
        invoking = true;
        operationFailure = null;
        await tauriBridge.invokeCandidate({
            sessionId: application.sessionId,
            requestId: rootSearch.requestId,
            extensionId: result.extensionId,
            entryId: result.entryId,
            actionId: result.actionId
        });
    }
    catch (error)
    {
        console.error("Action failed", error);
        operationFailure = "The action could not be completed. Try again.";
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
    // Pending work preserves the last coherent list. Its entries cannot be invoked
    // until the current request has produced results and Rust can validate them.
    if (next.phase === "ready")
    {
        hasCompletedSearch = true;
        if (import.meta.env.DEV)
        {
            observeSearch(next.requestId, "received");
        }
    }
    rootSearch = next.phase === "searching" ? { ...next, results: rootSearch.results } : next;
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
    navigation = next;
    viewInput.update(next);
    if (dismiss)
    {
        void tauriBridge.dismissLauncher().catch(fail);
    }
    reconcileViewInput();
}

function changeViewQuery(text: string): void
{
    viewInput.query(text);
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
    viewInputError = viewInput.inputError;
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

async function sendViewEvent(event: ViewEvent | null): Promise<number | null>
{
    const current = navigation.current;
    if (!application || !current)
    {
        return null;
    }
    const operation = ++viewOperation;
    const blocking = viewInput.begin(event);
    let receipt: ViewEventReceipt | null = null;
    reconcileViewInput();
    operationFailure = null;
    try
    {
        receipt = await submitViewEvent({
            sessionId: application.sessionId,
            routeId: current.routeId,
            revision: current.revision,
            operation: event === null ? { kind: "back" } : { kind: "event", event }
        });
        return receipt?.viewRevision ?? null;
    }
    catch (error)
    {
        if (operation === viewOperation)
        {
            console.error("View operation failed", error);
            operationFailure = "The action could not be completed. Try again.";
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
    // Product keys never fall through to implicit WebView behavior. Surface
    // handlers may still provide an explicit action after this capture phase.
    if (event.key === "Tab" || event.key === "F5")
    {
        event.preventDefault();
    }
}
</script>

<svelte:window onkeydowncapture={controlLauncherKeyboard} />

{#if failed}
    <main class="fatal" role="alert">
        <strong>Search is unavailable.</strong>
        <button onclick={() => window.location.reload()}>Try again</button>
    </main>
{:else}
    <svelte:boundary onerror={(error => fail(error))}>
        {#if navigation.current}
            {#key navigation.current.routeId}
                <ExtensionView
                    snapshot={navigation.current}
                    resourceOrigin={application?.resourceOrigin ?? ""}
                    busy={viewPending || viewInputError !== null}
                    error={viewInputError ?? operationFailure ?? navigationError}
                    onQuery={changeViewQuery}
                    onEvent={sendViewEvent}
                    onResume={resumeView}
                    onBack={() =>
                    {
                        void sendViewEvent(null);
                    }}
                />
            {/key}
        {:else}
            <RootSearch
                snapshot={rootSearch}
                {hasCompletedSearch}
                {refreshing}
                onRefresh={refreshSearch}
                onSettings={() =>
                {
                    void tauriBridge.openSettings().catch(error =>
                    {
                        console.error("Settings could not be opened", error);
                        operationFailure = "Settings could not be opened. Try again.";
                    });
                }}
                inputError={queryFailure}
                busy={invoking || refreshing || navigation.busy || !application
                || rootSearch.requestId !== latestRequestId
                || rootSearch.phase !== "ready"}
                onQuery={publishQuery}
                onDismiss={() =>
                {
                    tauriBridge.dismissLauncher().catch(fail);
                }}
                onInvoke={invokeCandidate}
            />
        {/if}
        {#if operationFailure && !navigation.current}
            <div class="operation-failure" role="alert">{operationFailure}</div>
        {/if}
        {#snippet failed()}
            <main class="fatal" role="alert">Search could not be displayed.</main>
        {/snippet}
    </svelte:boundary>
{/if}

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
