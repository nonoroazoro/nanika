<script lang="ts">
import DismissButton from "../components/ui/DismissButton.svelte";
const { message, onDismiss }: { message: string | null; onDismiss: () => void; } = $props();
</script>

<div class="toast-region" aria-live="polite" aria-atomic="true">
    {#if message}
        <div class="toast">
            <span>{message}</span>
            <DismissButton label="Dismiss notification" onclick={onDismiss} />
        </div>
    {/if}
</div>

<style>
.toast-region { position: absolute; z-index: 10; top: 48px; left: 50%; transform: translateX(-50%); width: max-content; max-width: calc(100% - 48px); pointer-events: none; }
.toast { display: flex; align-items: center; gap: var(--space-3); max-width: 32rem; padding: var(--space-2) var(--space-3); border: 1px solid var(--border-subtle); border-radius: var(--control-radius); background: var(--surface-window); color: var(--text-primary); box-shadow: var(--shadow-popup); font-size: var(--font-control); line-height: 1.4; pointer-events: auto; }
.toast span { overflow-wrap: anywhere; }
</style>
