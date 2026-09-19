<script lang="ts">
const { checked, label, id, disabled = false, onChange }: {
    checked: boolean;
    label: string;
    id?: string;
    disabled?: boolean;
    onChange: (checked: boolean) => void;
} = $props();
</script>

<button
    {id}
    type="button"
    class="switch"
    role="switch"
    aria-checked={checked}
    aria-label={label}
    {disabled}
    onclick={() => onChange(!checked)}
>
    <span class="track" aria-hidden="true"><span class="thumb"></span></span>
</button>

<style>
.switch { display: inline-flex; align-items: center; justify-content: center; width: 40px; min-width: 40px; min-height: 32px; padding: 4px; border: 0; background: transparent; border-radius: 6px; flex-shrink: 0; }
.track { width: 32px; height: 19px; border: 1px solid var(--border-window); border-radius: 10px; background: var(--surface-raised); transition: background-color 120ms ease, border-color 120ms ease; }
.thumb { display: block; width: 13px; height: 13px; margin: 2px; border-radius: 50%; background: var(--text-secondary); transition: transform 120ms ease, background-color 120ms ease; }
.switch[aria-checked="true"] .track { background: var(--accent); border-color: var(--accent); }
.switch[aria-checked="true"] .thumb { transform: translateX(13px); background: var(--accent-foreground); }
.switch:disabled { opacity: 0.45; cursor: default; }
.switch:focus-visible .track { border-color: var(--text-secondary); }
@media (prefers-reduced-motion: reduce) { .track, .thumb { transition: none; } }
</style>
