<script lang="ts">
import { isCompositionKey } from "../../ui/keyboard";
import { AlertDialog } from "bits-ui";
import { onDestroy } from "svelte";
import ActionSymbol from "../extensions/ActionSymbol.svelte";
import ExtensionIcon from "../extensions/ExtensionIcon.svelte";
import Button from "./Button.svelte";
import { confirmation, settleConfirmation } from "../../ui/confirmation";
import type { ConfirmationRequest } from "../../types/ConfirmationRequest";

let displayed = $state<ConfirmationRequest | null>(null);
let confirmButton = $state<HTMLButtonElement>();
let returnFocus: HTMLElement | null = null;
const open = $derived($confirmation !== null);
$effect(() =>
{
    if ($confirmation)
    {
        displayed = $confirmation;
        returnFocus = document.activeElement instanceof HTMLElement ? document.activeElement : null;
    }
});
onDestroy(() => settleConfirmation(false));

function _keydown(event: KeyboardEvent): void
{
    // Bits handles Escape at document level; other shortcuts stay inside the modal.
    if (event.key === "Escape")
    {
        return;
    }
    event.stopPropagation();
    if (
        event.key === "Enter"
        && (event.repeat || isCompositionKey(event) || event.metaKey || event.ctrlKey || event.altKey || event.shiftKey)
    )
    {
        event.preventDefault();
    }
}
</script>

<svelte:window onblur={() => settleConfirmation(false)} />
<svelte:document
    onvisibilitychange={() =>
    {
        if (document.visibilityState !== "visible")
        {
            settleConfirmation(false);
        }
    }}
/>

<AlertDialog.Root
    {open}
    onOpenChange={value =>
    {
        if (!value)
        {
            settleConfirmation(false);
        }
    }}
>
    <AlertDialog.Portal>
        <AlertDialog.Overlay class="confirmation-overlay" />
        <AlertDialog.Content
            class="confirmation-dialog"
            interactOutsideBehavior="close"
            onOpenAutoFocus={event =>
            {
                event.preventDefault();
                confirmButton?.focus({ preventScroll: true });
            }}
            onCloseAutoFocus={event =>
            {
                event.preventDefault();
                if (document.hasFocus() && returnFocus?.isConnected && !returnFocus.closest('[role="menu"]'))
                {
                    returnFocus.focus({ preventScroll: true });
                }
            }}
            onkeydown={_keydown}
        >
            <div class="confirmation-content">
                {#if displayed?.icon}
                    <span class="confirmation-icon" aria-hidden="true">
                        {#if displayed.icon.kind === "symbol"}<ActionSymbol name={displayed.icon.name} />
                        {:else}<ExtensionIcon src={displayed.icon.url} />{/if}
                    </span>
                {/if}
                <div class="confirmation-copy">
                    <AlertDialog.Title class="confirmation-title">{displayed?.title ?? ""}</AlertDialog.Title>
                    {#if displayed?.message}
                        <AlertDialog.Description class="confirmation-description">{
                            displayed.message
                        }</AlertDialog.Description>
                    {/if}
                </div>
            </div>
            <div class="confirmation-actions">
                <Button variant="outline" onclick={() => settleConfirmation(false)}>Cancel</Button>
                <Button
                    variant={displayed?.destructive ? "danger" : "primary"}
                    focusRing
                    bind:ref={confirmButton}
                    onclick={() => settleConfirmation(true)}
                >{displayed?.label ?? "Confirm"}</Button>
            </div>
        </AlertDialog.Content>
    </AlertDialog.Portal>
</AlertDialog.Root>

<style>
:global(.confirmation-overlay) { position: fixed; inset: 0; z-index: 40; background: rgb(0 0 0 / 18%); }
:global(.confirmation-dialog) { position: fixed; z-index: 41; inset: 0; margin: auto; width: min(20rem, calc(100vw - 2rem)); height: fit-content; max-height: calc(100vh - 2rem); overflow: auto; box-sizing: border-box; padding: 0; border: 1px solid var(--border-subtle); border-radius: var(--radius-window); background: linear-gradient(160deg, color-mix(in srgb, var(--surface-popup), var(--surface-form) 40%), var(--surface-popup)); color: var(--text-primary); box-shadow: 0 1px 2px rgb(0 0 0 / 6%), 0 8px 24px -8px rgb(0 0 0 / 18%), 0 24px 64px -24px rgb(0 0 0 / 28%), inset 0 1px 0 rgb(255 255 255 / 14%); outline: none; transform: scale(1); }
/* Bits retains both layers until their transitions finish, including interrupted exits. */
:global(.confirmation-dialog), :global(.confirmation-overlay) { opacity: 1; transition: opacity var(--motion-popup-enter) var(--motion-ease), transform var(--motion-popup-enter) var(--motion-ease); }
:global(.confirmation-dialog[data-starting-style]), :global(.confirmation-dialog[data-ending-style]) { opacity: 0; transform: scale(0.96); }
:global(.confirmation-overlay[data-starting-style]), :global(.confirmation-overlay[data-ending-style]) { opacity: 0; }
:global(.confirmation-dialog[data-ending-style]), :global(.confirmation-overlay[data-ending-style]) { transition-duration: var(--motion-enter); }
@media (prefers-reduced-motion: reduce) {
  :global(.confirmation-dialog) { transform: none !important; transition-property: opacity; }
}
:global(:root[data-ui-active="false"] .confirmation-dialog), :global(:root[data-ui-active="false"] .confirmation-overlay) { transition: none; }
:global(.confirmation-title) { margin: 0; font-size: 1rem; line-height: 1.4; font-weight: 600; overflow-wrap: anywhere; }
:global(.confirmation-description) { margin: var(--space-2) 0 0; font-size: var(--font-meta); line-height: 1.55; color: var(--text-secondary); overflow-wrap: anywhere; white-space: pre-line; }
.confirmation-actions { display: flex; justify-content: flex-end; flex-wrap: wrap; gap: var(--space-3); padding: var(--space-4) var(--space-5); border-top: 1px solid var(--border-subtle); background: color-mix(in srgb, var(--surface-popup), var(--surface-raised) 28%); }
.confirmation-actions :global(button) { min-width: 5rem; max-width: 100%; white-space: normal; overflow-wrap: anywhere; }

.confirmation-content { display: flex; align-items: flex-start; gap: var(--space-4); padding: var(--space-6) var(--space-5); }
.confirmation-copy { min-width: 0; flex: 1; }
.confirmation-icon { --icon-size: 2rem; --action-symbol-size: 1.375rem; display: grid; place-items: center; width: 2rem; height: 2rem; flex-shrink: 0; }
</style>
