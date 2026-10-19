<script lang="ts">
import { createAttachmentKey } from "svelte/attachments";
import { buttonMotion } from "../motion/style";
import { Button } from "bits-ui";

import type { ButtonProps } from "../../types/ButtonProps";

const motionKey = createAttachmentKey();

let {
    feedback = true,
    pressFeedback = feedback,
    focusRing = false,
    size = "content",
    ref = $bindable(),
    type = "button",
    variant = "ghost",
    class: className,
    children,
    ...attributes
}: ButtonProps = $props();
</script>

{#snippet control()}
    <Button.Root
        {...{ [motionKey]: feedback ? buttonMotion : undefined }}
        {...attributes}
        {type}
        bind:ref={() => ref ?? null, element =>
        {
            ref = element instanceof HTMLButtonElement ? element : undefined;
        }}
        class={["ui-button", variant !== "ghost" && "control-button", variant === "primary" && "primary", className]}
        data-variant={variant}
        data-size={size}
        data-press-feedback={pressFeedback ? "" : undefined}
        data-focus-ring={focusRing ? "" : undefined}
    >
        {@render children?.()}
    </Button.Root>
{/snippet}

{#if focusRing}
    <span class="button-focus-frame" data-variant={variant}>
        {@render control()}
    </span>
{:else}
    {@render control()}
{/if}

<style>
/* CSS owns press geometry; the attachment owns colors. Native :active also
   releases on cancelled input, without capturing pointers or delaying clicks. */
:global(.ui-button[data-press-feedback]) { transform: scale(1); transition: transform var(--motion-enter) var(--motion-ease); }
:global(.ui-button[data-press-feedback]:active:not(:disabled, [aria-disabled="true"])) { transform: scale(0.97); }
:global(.ui-button[data-size="action"]) { min-inline-size: 6rem; max-inline-size: 100%; padding-inline: var(--space-3); flex-shrink: 0; }
:global(:root[data-ui-active="false"] .ui-button[data-press-feedback]) { transform: none; transition: none; }
@media (prefers-reduced-motion: reduce) {
  :global(.ui-button[data-press-feedback]) { transform: none !important; transition: none; }
}
:global(.ui-button[data-variant="danger"]) { color: var(--danger-foreground); border-color: var(--danger-solid); background: var(--danger-solid); }
:global(.ui-button[data-variant="danger"]:hover:not(:disabled, [aria-disabled="true"])) { border-color: var(--danger-solid-hover); background: var(--danger-solid-hover); }
:global(.ui-button[data-variant="danger"]:active:not(:disabled, [aria-disabled="true"])) { border-color: var(--danger-solid-pressed); background: var(--danger-solid-pressed); }
/* The frame stays still while its button presses inward. It never intercepts input. */
.button-focus-frame { display: inline-flex; min-width: 0; max-width: 100%; border-radius: var(--control-radius); }
.button-focus-frame > :global(button) { width: 100%; }
.button-focus-frame:has(> :global(:focus)) { outline: 1px solid var(--accent); outline-offset: 2px; }
.button-focus-frame[data-variant="danger"]:has(> :global(:focus)) { outline-color: var(--danger-solid); }
/* Window deactivation changes control presentation, not destructive semantics. */
:global(:root[data-ui-active="false"] .ui-button[data-variant="danger"]) { color: var(--control-foreground); border-color: var(--control-border); background: var(--control-surface); }
:global(:root[data-ui-active="false"] .ui-button[data-variant="danger"]:hover:not(:disabled, [aria-disabled="true"])) { border-color: var(--control-border-hover); background: var(--control-surface-hover); }
</style>
