<script lang="ts">
  import { onMount } from 'svelte'

  import { tauriBridge } from './bridge'
  import { RootSearch } from './components'
  import type { ApplicationSnapshot, RootSearchSnapshot, SearchResult } from './types'

  let application = $state<ApplicationSnapshot | null>(null)
  let failure = $state<string | null>(null)
  let pendingRootSearch: RootSearchSnapshot | null = null

  onMount(() => {
    void (async () => {
      try {
        const initial = await tauriBridge.openSession(updateRootSearch)
        application = {
          ...initial,
          rootSearch:
            pendingRootSearch && pendingRootSearch.generation >= initial.rootSearch.generation
              ? pendingRootSearch
              : initial.rootSearch,
        }
        pendingRootSearch = null
      } catch (error) {
        failure = error instanceof Error ? error.message : String(error)
      }
    })()
  })

  async function publishQuery(query: string): Promise<void> {
    try {
      const next = await tauriBridge.publishQuery(query)
      const current = application?.rootSearch
      if (!current || next.generation >= current.generation) {
        updateRootSearch(next)
      }
    } catch (error) {
      failure = error instanceof Error ? error.message : String(error)
    }
  }

  async function invokeCandidate(result: SearchResult): Promise<void> {
    try {
      const keepsLauncherOpen = await tauriBridge.invokeCandidate({
        extensionId: result.extensionId,
        entryId: result.entryId,
        actionId: result.actionId,
      })
      if (!keepsLauncherOpen) {
        await tauriBridge.dismissLauncher()
      }
    } catch (error) {
      failure = error instanceof Error ? error.message : String(error)
    }
  }

  function updateRootSearch(rootSearch: RootSearchSnapshot): void {
    if (application) {
      if (rootSearch.generation >= application.rootSearch.generation) {
        application = { ...application, rootSearch }
      }
      return
    }
    if (!pendingRootSearch || rootSearch.generation >= pendingRootSearch.generation) {
      pendingRootSearch = rootSearch
    }
  }
</script>

{#if application}
  <RootSearch
    snapshot={application.rootSearch}
    onQuery={publishQuery}
    onDismiss={() => tauriBridge.dismissLauncher()}
    onInvoke={invokeCandidate}
  />
{:else if failure}
  <main class="fatal" role="alert">
    <strong>Nanika could not start.</strong>
    <span>{failure}</span>
  </main>
{:else}
  <main class="loading" aria-label="Nanika is starting"></main>
{/if}

<style>
  .loading,
  .fatal {
    width: 100%;
    height: 100%;
    border: 1px solid var(--border-window);
    border-radius: var(--radius-window);
    background: var(--surface-window);
    box-shadow: var(--shadow-window);
  }

  .fatal {
    display: grid;
    place-content: center;
    gap: var(--space-2);
    padding: var(--space-6);
    color: var(--text-primary);
    text-align: center;
  }

  .fatal span {
    color: var(--text-secondary);
    font-size: var(--font-meta);
  }
</style>
