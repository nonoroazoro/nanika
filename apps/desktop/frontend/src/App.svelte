<script lang="ts">
import { onMount, tick } from "svelte";

import { tauriBridge } from "./bridge";
import { ExtensionView, RootSearch } from "./components";
import type { ApplicationSnapshot, NavigationSnapshot, RootSearchSnapshot, SearchResult, ViewEvent } from "./types";
import type { SearchObservation } from "./development";

let application = $state<ApplicationSnapshot | null>(null);
let failure = $state<string | null>(null);
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
let desiredViewQuery = $state<string | null>(null);
let submittedNavigationRevision = 0;
let viewOperation = 0;
let viewResumeRequested = false;
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
    failure = error instanceof Error ? error.message : String(error);
    if (import.meta.env.DEV)
    {
        observeSearch(latestRequestId, "failed");
    }
    console.error("Search window failed", failure);
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
            queryFailure = error instanceof Error ? error.message : String(error);
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
        operationFailure = error instanceof Error ? error.message : String(error);
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
        operationFailure = error instanceof Error ? error.message : String(error);
        console.error("Action failed", operationFailure);
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
        fail(next.error ?? "Search failed. Reload the window to reconnect.");
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
    const changedRoute = next.current?.routeId !== navigation.current?.routeId;
    const dismiss = next.dismissCount > navigation.dismissCount;
    navigation = next;
    if (changedRoute)
    {
        desiredViewQuery = next.current?.view.kind === "list" ? next.current.view.list.search_text : null;
        viewInputError = null;
        viewResumeRequested = false;
    }
    if (viewPending && !next.busy && next.revision > submittedNavigationRevision)
    {
        viewPending = false;
    }
    if (dismiss)
    {
        void tauriBridge.dismissLauncher().catch(fail);
    }
    if (!next.error)
    {
        flushViewQuery();
        flushViewResume();
    }
}

function changeViewQuery(text: string): void
{
    desiredViewQuery = text;
    viewInputError = [...text].length > 4096
        ? "View search supports up to 4096 characters. Edit the input to continue."
        : null;
    flushViewQuery();
}

function flushViewQuery(): void
{
    const current = navigation.current;
    if (
        viewPending || navigation.busy || viewInputError || !current || current.view.kind !== "list"
        || desiredViewQuery === null || desiredViewQuery === current.view.list.search_text
    )
    {
        return;
    }
    void sendViewEvent({ kind: "searchChanged", text: desiredViewQuery });
}

function resumeView(): void
{
    viewResumeRequested = true;
    flushViewResume();
}

function flushViewResume(): void
{
    if (viewPending || navigation.busy || !application || !navigation.current || !viewResumeRequested)
    {
        return;
    }
    viewResumeRequested = false;
    void sendViewEvent({ kind: "resumed" });
}

async function sendViewEvent(event: ViewEvent | null): Promise<void>
{
    const current = navigation.current;
    const nonBlocking = event?.kind === "selectionChanged" || event?.kind === "resumed";
    if (!application || !current)
    {
        return;
    }
    const operation = ++viewOperation;
    submittedNavigationRevision = navigation.revision;
    if (!nonBlocking)
    {
        viewPending = true;
    }
    operationFailure = null;
    try
    {
        await tauriBridge.viewEvent({
            sessionId: application.sessionId,
            routeId: current.routeId,
            revision: current.revision,
            operation: event === null ? { kind: "back" } : { kind: "event", event }
        });
    }
    catch (error)
    {
        if (operation === viewOperation)
        {
            if (!nonBlocking)
            {
                viewPending = false;
            }
            operationFailure = error instanceof Error ? error.message : String(error);
            flushViewResume();
        }
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

{#if failure}
    <main class="fatal" role="alert">
        <strong>Search is unavailable.</strong>
        <span>{failure}</span>
        <button onclick={() => window.location.reload()}>Reload window</button>
    </main>
{:else}
    <svelte:boundary onerror={(error => fail(error))}>
        {#if navigation.current}
            {#key navigation.current.routeId}
                <ExtensionView
                    snapshot={navigation.current}
                    resourceOrigin={application?.resourceOrigin ?? ""}
                    busy={viewPending || viewInputError !== null}
                    error={viewInputError ?? operationFailure ?? navigation.error}
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
  border: 1px solid var(--border-window);
  border-radius: var(--radius-window);
  background: var(--surface-window);
  box-shadow: var(--shadow-window);
  color: var(--text-primary);
  text-align: center;
}

.fatal span {
  color: var(--text-secondary);
  font-size: var(--font-meta);
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
