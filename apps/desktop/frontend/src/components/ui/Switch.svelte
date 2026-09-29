<script lang="ts">
import { buttonMotion, styleMotion } from "../motion/style";
import { createAttachmentKey } from "svelte/attachments";
import { Switch } from "bits-ui";

const motionKey = createAttachmentKey();
const trackMotion = styleMotion({
    properties: ["background-color", "border-color"],
    scope: ".ui-switch",
    duration: "--motion-switch",
    ease: "ease-out"
});
const thumbMotion = styleMotion({
    properties: ["transform", "background-color"],
    scope: ".ui-switch",
    duration: "--motion-switch",
    ease: "ease-out"
});

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
    bind:checked={() => checked, onChange}
    disabled={disabled || busy}
    aria-busy={busy || undefined}
    aria-label={label}
    class="ui-switch"
    {...{ [motionKey]: buttonMotion }}
>
    <span {@attach trackMotion} class="track" aria-hidden="true"><Switch.Thumb
            {...{ [motionKey]: thumbMotion }}
            class="switch-thumb"
        /></span>
</Switch.Root>

<style>
:global(.ui-switch) { display: inline-flex; align-items: center; justify-content: center; width: 40px; min-width: 40px; min-height: var(--control-height); padding: 4px; border: 0; background: transparent; border-radius: var(--control-radius); flex-shrink: 0; }
.track { pointer-events: none; width: var(--switch-track-width); height: var(--switch-track-height); border: var(--switch-stroke) solid var(--switch-border); border-radius: 10px; background: var(--switch-surface); }
:global(.switch-thumb) { display: block; width: var(--switch-thumb-size); height: var(--switch-thumb-size); margin: var(--switch-inset); border-radius: 50%; background: var(--switch-thumb); }
:global(.ui-switch[data-state="checked"]) .track { background: var(--switch-active); border-color: var(--switch-active); }
:global(.switch-thumb[data-state="checked"]) { transform: translateX(var(--switch-thumb-travel)); background: var(--switch-thumb-on); }
:global(.ui-switch:hover:not(:disabled)), :global(.ui-switch:focus-visible) { background: transparent; }
:global(.ui-switch[data-state="checked"]:hover:is(:enabled, [aria-busy="true"])) .track { background: var(--switch-active-hover); border-color: var(--switch-active-hover); }
:global(.ui-switch[data-state="unchecked"]:hover:is(:enabled, [aria-busy="true"]) .switch-thumb) { background: var(--switch-thumb-hover); }
:global(.ui-switch[data-state="checked"]:active:enabled) .track { background: var(--switch-active-pressed); border-color: var(--switch-active-pressed); }
:global(.ui-switch:disabled) { opacity: 0.45; cursor: default; }
/* Pending work blocks repeat activation without changing the pointer or fading the control. */
:global(.ui-switch[aria-busy="true"]) { opacity: 1; cursor: pointer; }
</style>
