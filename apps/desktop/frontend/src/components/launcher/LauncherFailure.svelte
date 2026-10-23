<script lang="ts">
import { onDestroy } from "svelte";
import { tauriBridge } from "../../bridge";
import { lifecycleFailure } from "../../localization/lifecycle-errors";
import { operationErrorText, operationFailure } from "../../localization/operation-errors";
import { localization } from "../../localization/state.svelte";
import Button from "../ui/Button.svelte";

import type { OperationError } from "../../generated/OperationError";

const { error, startup, sessionId }: { error: OperationError; startup: boolean; sessionId: number | null; } = $props();
let diagnostic = $state<string | null>(null);
let diagnosticError = $state<OperationError | null>(null);
let expanded = $state(false);
let loading = $state(false);
let disposed = false;
const diagnosticAction = $derived.by(() =>
{
    if (loading)
    {
        return "launcher-loading-diagnostics";
    }
    return expanded ? "launcher-hide-diagnostics" : "launcher-show-diagnostics";
});

onDestroy(() =>
{
    disposed = true;
});

async function _toggleDiagnostic(): Promise<void>
{
    expanded = !expanded;
    if (!expanded || diagnostic !== null || loading || sessionId === null)
    {
        return;
    }
    const owner = sessionId;
    loading = true;
    diagnosticError = null;
    try
    {
        const retained = lifecycleFailure(await tauriBridge.readStartupDiagnostic(owner));
        if (!disposed && sessionId === owner)
        {
            diagnostic = retained.diagnostic ?? "";
        }
    }
    catch (failure)
    {
        if (!disposed && sessionId === owner)
        {
            diagnosticError = operationFailure(failure, "startupDiagnosticUnavailable");
        }
    }
    finally
    {
        if (!disposed)
        {
            loading = false;
        }
    }
}
</script>

<main class="launcher-failure">
    <div role="alert" class="explanation">
        <strong>{localization.text({ message: "launcher-unavailable" })}</strong>
        <p>{operationErrorText(error, localization.locale, value => localization.text(value))}</p>
    </div>
    {#if startup}
        <p class="recovery">{localization.text({ message: "launcher-startup-recovery" })}</p>
        <Button
            variant="outline"
            focusRing
            disabled={loading || sessionId === null}
            aria-expanded={expanded}
            aria-controls="startup-diagnostic"
            onclick={_toggleDiagnostic}
        >
            {
                localization.text({
                    message: diagnosticAction
                })
            }
        </Button>
        <div id="startup-diagnostic" hidden={!expanded}>
            {#if diagnosticError}
                <p role="alert">
                    {operationErrorText(diagnosticError, localization.locale, value => localization.text(value))}
                </p>
            {:else if diagnostic !== null}
                <textarea
                    readonly
                    aria-label={localization.text({ message: "launcher-diagnostics" })}
                    value={diagnostic || localization.text({ message: "launcher-no-diagnostics" })}
                ></textarea>
            {/if}
        </div>
    {:else}
        <Button variant="outline" focusRing onclick={() => window.location.reload()}>{
            localization.text({ message: "action-retry" })
        }</Button>
    {/if}
</main>

<style>
.launcher-failure {
  display: flex;
  flex-direction: column;
  align-items: center;
  justify-content: safe center;
  gap: var(--space-3);
  height: 100%;
  padding: var(--space-6);
  overflow: auto;
  background: var(--surface-window);
  color: var(--text-primary);
  text-align: center;
}
.explanation, .recovery, #startup-diagnostic { max-width: 100%; }
.explanation p, #startup-diagnostic p { white-space: pre-wrap; overflow-wrap: anywhere; }
.explanation p { margin-top: var(--space-2); }
.recovery { color: var(--text-secondary); font-size: var(--font-meta); }
#startup-diagnostic { width: 100%; min-height: 0; }
textarea {
  height: 30vh;
  width: 100%;
  resize: none;
  margin: 0;
  padding: var(--space-3);
  overflow: auto;
  border: 1px solid var(--border-window);
  border-radius: var(--radius-row);
  background: var(--surface-raised);
  font-size: var(--font-meta);
  white-space: pre-wrap;
  overflow-wrap: anywhere;
  text-align: left;
  user-select: text;
}
</style>
