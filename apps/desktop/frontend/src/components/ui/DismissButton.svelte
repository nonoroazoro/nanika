<script lang="ts">
import { styleMotion } from "../motion/style";
import Button from "./Button.svelte";
import XIcon from "../icons/XIcon.svelte";

import type { DismissButtonProps } from "../../types/DismissButtonProps";

const glyphMotion = styleMotion({ properties: ["opacity"], scope: ".dismiss-button" });

const { label, class: className, ...attributes }: DismissButtonProps = $props();
</script>

<Button
    {...attributes}
    class={["dismiss-button", className]}
    aria-label={label}
    title={attributes.title ?? label}
>
    <span {@attach glyphMotion} class="dismiss-glyph" aria-hidden="true"><XIcon size={12} strokeWidth={2} /></span>
</Button>

<style>
:global(.ui-button.dismiss-button) { display: inline-grid; flex: 0 0 24px; width: 24px; height: 24px; min-height: 24px; place-items: center; padding: 0; border: 0; border-radius: 50%; background: transparent; color: var(--text-secondary); }
:global(.ui-button.dismiss-button:is(:hover, :focus-visible):not(:disabled, [aria-disabled="true"])) { background: transparent; color: var(--text-primary); }
:global(.ui-button.dismiss-button:active:not(:disabled, [aria-disabled="true"])) { background: transparent; color: var(--text-primary); }
.dismiss-glyph { display: grid; width: 16px; height: 16px; place-items: center; border-radius: 50%; background: currentColor; pointer-events: none; }
:global(.ui-button.dismiss-button:active:not(:disabled, [aria-disabled="true"])) .dismiss-glyph { opacity: 0.72; }
.dismiss-glyph :global(svg) { color: var(--surface-form); }
</style>
