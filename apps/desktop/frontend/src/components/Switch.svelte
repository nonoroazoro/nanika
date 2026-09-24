<script lang="ts">
import { Switch } from "bits-ui";

const { checked, label, id, disabled = false, busy = false, onChange }: {
    checked: boolean;
    label: string;
    id?: string;
    disabled?: boolean;
    busy?: boolean;
    onChange: (checked: boolean) => void;
} = $props();
</script>

<Switch.Root
    {id}
    {checked}
    disabled={disabled || busy}
    aria-busy={busy || undefined}
    aria-label={label}
    onCheckedChange={onChange}
    class="ui-switch"
>
    <span class="track" aria-hidden="true"><Switch.Thumb class="switch-thumb" /></span>
</Switch.Root>

<style>
:global(.ui-switch) { display: inline-flex; align-items: center; justify-content: center; width: 40px; min-width: 40px; min-height: var(--control-height); padding: 4px; border: 0; background: transparent; border-radius: var(--control-radius); flex-shrink: 0; }
.track { pointer-events: none; width: 32px; height: 19px; border: 1px solid var(--border-window); border-radius: 10px; background: var(--surface-raised); transition: background-color var(--motion-switch) ease-out, border-color var(--motion-switch) ease-out; }
:global(.switch-thumb) { display: block; width: 13px; height: 13px; margin: 2px; border-radius: 50%; background: var(--text-secondary); transition: transform var(--motion-switch) ease-out, background-color var(--motion-switch) ease-out; }
:global(.ui-switch[data-state="checked"]) .track { background: var(--accent); border-color: var(--accent); }
:global(.switch-thumb[data-state="checked"]) { transform: translateX(13px); background: var(--accent-foreground); }
:global(.ui-switch:hover:not(:disabled)), :global(.ui-switch:focus-visible) { background: transparent; }
:global(.ui-switch[data-state="checked"]:hover:is(:enabled, [aria-busy="true"])) .track { background: var(--accent-hover); border-color: var(--accent-hover); }
:global(.ui-switch[data-state="unchecked"]:hover:is(:enabled, [aria-busy="true"]) .switch-thumb) { background: var(--text-primary); }
:global(.ui-switch:disabled) { opacity: 0.45; cursor: default; }
/* Pending work blocks repeat activation without changing the pointer or fading the control. */
:global(.ui-switch[aria-busy="true"]) { opacity: 1; cursor: pointer; }
</style>
