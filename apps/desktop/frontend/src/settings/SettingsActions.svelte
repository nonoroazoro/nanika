<script lang="ts">
import Button from "../components/Button.svelte";
const { dirty, saving, error, disabled = false, onDiscard }: {
    dirty: boolean;
    saving: boolean;
    error?: string | null;
    disabled?: boolean;
    onDiscard: () => void;
} = $props();
</script>

{#if dirty || saving || error}
    <footer class="settings-actions">
        <div class="feedback">{#if error}<p role="alert">{error}</p>{/if}</div>
        {#if dirty || saving}
            <Button variant="outline" disabled={saving} onclick={onDiscard}>Discard</Button>
            <Button variant="primary" type="submit" disabled={saving || disabled}>
                {saving ? "Saving…" : "Save changes"}
            </Button>
        {/if}
    </footer>
{/if}

<style>
.settings-actions { position: sticky; bottom: 0; display: flex; align-items: center; gap: 8px; flex-shrink: 0; margin-top: auto; border-top: 1px solid var(--border-subtle); padding: 14px 0; background: var(--surface-window); }
.feedback { flex: 1; min-width: 0; }
p { margin: 0; color: var(--text-danger); font-size: var(--font-control); line-height: 1.5; overflow-wrap: anywhere; }
.settings-actions :global(.ui-button) { flex-shrink: 0; }
</style>
