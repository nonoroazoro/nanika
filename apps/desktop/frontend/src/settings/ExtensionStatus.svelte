<script lang="ts">
import { untrack } from "svelte";
import type { ExtensionSettings } from "../types/Settings";

const { lifecycleState }: { lifecycleState: ExtensionSettings["state"]; } = $props();
const labels = {
    disabled: "Disabled",
    dormant: "Ready on demand",
    starting: "Starting…",
    ready: "Running",
    stopping: "Stopping…",
    failed: "Unavailable"
} satisfies Record<ExtensionSettings["state"], string>;
let settled = $state(untrack(() => lifecycleState));
let showPending = $state(false);
const pending = $derived(lifecycleState === "starting" || lifecycleState === "stopping");
const displayed = $derived(pending && !showPending ? settled : lifecycleState);

// Delay only transient presentation. Authoritative state, terminal results and
// failures are never delayed, and a completed operation cancels its pending label.
$effect(() =>
{
    const current = lifecycleState;
    showPending = false;
    if (current !== "starting" && current !== "stopping")
    {
        settled = current;
        return;
    }
    const timer = setTimeout(() =>
    {
        showPending = true;
    }, 1000);
    return () => clearTimeout(timer);
});
</script>

<p role="status" aria-atomic="true" data-state={displayed}>
    <span class="dot" aria-hidden="true"></span>
    <span>{labels[displayed]}</span>
</p>

<style>
p { display: flex; align-items: center; gap: 0.375rem; margin: var(--space-1) 0 0; color: var(--text-secondary); font-size: var(--settings-description-size); line-height: var(--settings-description-line-height); }
.dot { flex: 0 0 var(--status-dot-size); width: var(--status-dot-size); height: var(--status-dot-size); border: 1px solid transparent; border-radius: 50%; background: var(--status-inactive); transition: background-color var(--motion-control) var(--motion-ease); }
p[data-state="ready"] .dot, p[data-state="dormant"] .dot { background: var(--status-available); }
p[data-state="failed"] .dot { background: var(--status-failed); }
</style>
