<script lang="ts">
import { onMount } from "svelte";

import { tauriBridge } from "./bridge";
import { RootSearch } from "./components";
import type { ApplicationSnapshot, RootSearchSnapshot, SearchResult } from "./types";

let application = $state<ApplicationSnapshot | null>(null);
let failure = $state<string | null>(null);
let queryFailure = $state<string | null>(null);
let invoking = $state(false);
let operationFailure = $state<string | null>(null);
let rootSearch = $state<RootSearchSnapshot>({
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
    console.error("Search window failed", failure);
}

async function publishQuery(query: string): Promise<void>
{
    operationFailure = null;
    const requestId = ++latestRequestId;
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
        }
    }
}

async function invokeCandidate(result: SearchResult): Promise<void>
{
    if (invoking || !application || rootSearch.requestId !== latestRequestId || rootSearch.phase !== "ready")
    {
        return;
    }
    try
    {
        invoking = true;
        operationFailure = null;
        const keepsLauncherOpen = await tauriBridge.invokeCandidate({
            sessionId: application.sessionId,
            requestId: rootSearch.requestId,
            extensionId: result.extensionId,
            entryId: result.entryId,
            actionId: result.actionId
        });
        if (!keepsLauncherOpen)
        {
            await tauriBridge.dismissLauncher();
        }
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
        || next.requestId !== latestRequestId
        || next.revision <= lastRevision
    )
    {
        return;
    }
    lastRevision = next.revision;
    if (next.phase === "error")
    {
        fail(next.error ?? "Search failed. Reload the window to reconnect.");
        return;
    }
    // Pending work preserves the last coherent list. Its entries cannot be invoked
    // until the current request has produced results and Rust can validate them.
    rootSearch = next.phase === "searching" ? { ...next, results: rootSearch.results } : next;
}
</script>

{#if failure}
    <main class="fatal" role="alert">
        <strong>Search is unavailable.</strong>
        <span>{failure}</span>
        <button onclick={() => window.location.reload()}>Reload window</button>
    </main>
{:else}
    <svelte:boundary onerror={(error => fail(error))}>
        <RootSearch
            snapshot={rootSearch}
            inputError={queryFailure}
            busy={invoking || !application
            || rootSearch.requestId !== latestRequestId
            || rootSearch.phase !== "ready"}
            onQuery={publishQuery}
            onDismiss={() =>
            {
                tauriBridge.dismissLauncher().catch(fail);
            }}
            onInvoke={invokeCandidate}
        />
        {#if operationFailure}
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
