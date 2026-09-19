<script lang="ts">
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
            <button class="secondary" type="button" disabled={saving} onclick={onDiscard}>Discard</button>
            <button class="primary" type="submit" disabled={saving || disabled}>
                {saving ? "Saving…" : "Save changes"}
            </button>
        {/if}
    </footer>
{/if}

<style>
.settings-actions { position: sticky; bottom: 0; display: flex; align-items: center; gap: 8px; flex-shrink: 0; margin-top: auto; border-top: 1px solid var(--border-subtle); padding: 14px 0; background: var(--surface-window); }
.feedback { flex: 1; min-width: 0; }
p { margin: 0; color: var(--text-danger); font-size: 12px; line-height: 1.5; overflow-wrap: anywhere; }
button { min-height: 32px; flex-shrink: 0; padding: 6px 14px; border-radius: 6px; font-size: 12px; }
.secondary { border: 1px solid var(--border-window); background: var(--surface-form); }
.secondary:hover:not(:disabled) { border-color: var(--border-window); background: var(--surface-raised); }
.primary { border: 1px solid var(--accent); background: var(--accent); color: var(--accent-foreground); }
.primary:hover:not(:disabled) { border-color: var(--accent); background: color-mix(in srgb, var(--accent) 90%, var(--text-primary)); }
.secondary:focus-visible { background: var(--surface-raised); }
.primary:focus-visible { background: color-mix(in srgb, var(--accent) 85%, var(--text-primary)); }
</style>
