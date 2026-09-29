<script lang="ts">
import { createAttachmentKey } from "svelte/attachments";
import { menuMotion } from "./popup-motion";
import { ContextMenu as Menu } from "bits-ui";
import ScrollArea from "./ScrollArea.svelte";
import { uiActivity } from "../../ui/activity";
import ShortcutKeys from "./ShortcutKeys.svelte";

import type { Snippet } from "svelte";
import type { Action } from "../../types/Action";

const motionKey = createAttachmentKey();

const { actions, position, anchor, open, heading, footer, shortcuts = {}, onClose, onClosed, onInvoke }: {
    actions: Action[];
    position: [number, number] | null;
    anchor?: HTMLElement;
    open: boolean;
    heading?: string;
    footer?: Snippet;
    shortcuts?: Record<string, string[]>;
    onClose: () => void;
    onClosed: () => void;
    onInvoke: (id: string, confirmed: boolean) => void;
} = $props();
let confirmation = $state.raw<Action | null>(null);
let outsideTarget: Element | null = null;
let returnFocus: HTMLElement | null = null;
const reference = $derived(
    anchor ?? {
        getBoundingClientRect: () => new DOMRect(position?.[0] ?? 16, position?.[1] ?? 16, 0, 0)
    }
);
// Align the popup to the trigger margin box while its icon keeps its own inset.
const attachedAlignOffset = $derived(anchor ? -Number.parseFloat(getComputedStyle(anchor).marginInlineStart) : 0);
const ordered = $derived(
    [...actions].sort((a, b) => Number(a.style === "destructive") - Number(b.style === "destructive"))
);

function _activate(action: Action): void
{
    if (!open || !action.enabled)
    {
        return;
    }
    if (action.confirmation_title && confirmation !== action)
    {
        confirmation = action;
        return;
    }
    onInvoke(action.id, confirmation === action);
}

function _keydown(event: KeyboardEvent): void
{
    // Let Bits handle Escape before window handlers; keep other keys inside the menu.
    if (event.key === "Escape")
    {
        return;
    }
    event.stopPropagation();
    if (event.key === "Tab")
    {
        event.preventDefault();
        onClose();
    }
}
</script>

<Menu.Root
    {open}
    onOpenChange={next =>
    {
        if (!next)
        {
            onClose();
        }
    }}
    onOpenChangeComplete={next =>
    {
        if (!next)
        {
            confirmation = null;
            onClosed();
        }
    }}
>
    {#if $uiActivity.visible && $uiActivity.focused}
        {#key position ?? anchor}
            <Menu.Portal>
                <Menu.Content
                    {...{ [motionKey]: menuMotion }}
                    id="context-menu"
                    class="popup-surface context-menu"
                    aria-label={heading ?? "Actions"}
                    customAnchor={reference}
                    data-attached={anchor ? "" : undefined}
                    side={anchor ? "top" : "bottom"}
                    align="start"
                    alignOffset={attachedAlignOffset}
                    sideOffset={anchor ? -anchor.offsetHeight : 4}
                    collisionPadding={8}
                    trapFocus={false}
                    preventScroll={false}
                    onOpenAutoFocus={() =>
                    {
                        outsideTarget = null;
                        const active = document.activeElement;
                        // Retargeting an open menu must preserve the original editing
                        // focus, not capture an item in the menu being replaced.
                        if (active instanceof HTMLElement && active !== document.body && !active.closest('[role="menu"]'))
                        {
                            returnFocus = active;
                        }
                    }}
                    onCloseAutoFocus={event =>
                    {
                        event.preventDefault();
                        if (
                            !open && document.hasFocus() && document.visibilityState === "visible"
                            && returnFocus?.isConnected
                            && !outsideTarget?.closest("input, textarea, select, button, a[href], [contenteditable=true]")
                        )
                        {
                            returnFocus.focus({ preventScroll: true });
                        }
                    }}
                    onInteractOutside={event =>
                    {
                        outsideTarget = event.target instanceof Element ? event.target : null;
                    }}
                    onkeydown={_keydown}
                    oncontextmenu={event => event.preventDefault()}
                    style={`--menu-anchor-inset: ${-attachedAlignOffset}px; --menu-anchor-width: ${
                        anchor?.offsetWidth ?? 0
                    }px; --menu-anchor-height: ${anchor?.offsetHeight ?? 0}px; width: ${
                        anchor ? "17.5rem" : "16rem"
                    }; max-height: min(var(--bits-context-menu-content-available-height), calc(100vh - 16px));`}
                >
                    <ScrollArea
                        style="max-height: min(calc(var(--bits-context-menu-content-available-height) - var(--menu-footer-height, 0px) - 2 * var(--popup-padding) - 2px), calc(100vh - 16px - var(--menu-footer-height, 0px) - 2 * var(--popup-padding) - 2px)); flex: none;"
                    >
                        <Menu.Group class={anchor ? "attached-actions" : undefined}>
                            {#if heading && !(anchor && footer)}<Menu.GroupHeading class="popup-heading">{
                                    heading
                                }</Menu.GroupHeading>{/if}
                            {#each ordered as action, index (action.id)}
                                {#if index > 0 && (action.group !== ordered[index - 1]?.group
    || (action.style === "destructive") !== (ordered[index - 1]?.style === "destructive"))}
                                    <Menu.Separator class="popup-separator" />
                                {/if}
                                {@const keys = shortcuts[action.id]}
                                <Menu.Item
                                    class="popup-item"
                                    disabled={!action.enabled}
                                    data-destructive={action.style === "destructive" ? "" : undefined}
                                    data-confirming={confirmation === action ? "" : undefined}
                                    closeOnSelect={false}
                                    onSelect={() => _activate(action)}
                                >
                                    <span>{confirmation === action ? action.confirmation_title : action.title}</span>
                                    {#if keys}<ShortcutKeys {keys} />{/if}
                                </Menu.Item>
                            {/each}
                        </Menu.Group>
                    </ScrollArea>
                    {#if anchor && footer}{@render footer()}{/if}
                </Menu.Content>
            </Menu.Portal>
        {/key}
    {/if}
</Menu.Root>

<style>
/* The menu keeps one scale/fade recipe; its origin follows the invocation. */
:global(.context-menu) { transform: scale(1); box-shadow: var(--shadow-menu); }
:global(.context-menu[data-state="closed"]) { opacity: 0; transform: scale(0.96); }
:global(.context-menu[data-attached]) { --menu-footer-height: calc(var(--menu-anchor-height) - 2px); padding: 0; transform-origin: left bottom; }
:global(.attached-actions .popup-item) { min-height: 2.5rem; font-size: var(--settings-label-size); }
:global(.attached-actions) { padding: var(--popup-padding); border-bottom: 1px solid var(--border-subtle); }
</style>
