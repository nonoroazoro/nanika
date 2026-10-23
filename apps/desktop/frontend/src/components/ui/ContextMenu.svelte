<script lang="ts">
import { localization } from "../../localization/state.svelte";
import { createAttachmentKey } from "svelte/attachments";
import { menuMotion } from "./popup-motion";
import { ContextMenu as Menu } from "bits-ui";
import ScrollArea from "./ScrollArea.svelte";
import { confirmationPending } from "../../ui/confirmation";
import { uiActivity } from "../../ui/activity";
import CheckIcon from "../icons/CheckIcon.svelte";
import type { ContextMenuSection } from "../../types/ContextMenuSection";
import ShortcutKeys from "./ShortcutKeys.svelte";

import type { Snippet } from "svelte";
import type { Action } from "../../generated/Action";
import type { LocalizedText } from "../../generated/LocalizedText";

const motionKey = createAttachmentKey();

const { actions, sections, position, anchor, open, heading, footer, shortcuts = {}, text, onClose, onClosed, onInvoke }:
    {
        text: (value: LocalizedText) => string;
        actions: Action[];
        sections?: ContextMenuSection[];
        position: [number, number] | null;
        anchor?: HTMLElement;
        open: boolean;
        heading?: string;
        footer?: Snippet;
        shortcuts?: Record<string, string[]>;
        onClose: () => void;
        onClosed: () => void;
        onInvoke: (id: string) => void;
    } = $props();
let outsideTarget: Element | null = null;
let returnFocus: HTMLElement | null = null;
const reference = $derived(
    anchor ?? {
        getBoundingClientRect: () => new DOMRect(position?.[0] ?? 16, position?.[1] ?? 16, 0, 0)
    }
);
// Align the popup to the trigger margin box while its icon keeps its own inset.
const attachedAlignOffset = $derived(anchor ? -Number.parseFloat(getComputedStyle(anchor).marginInlineStart) : 0);
const displayedSections = $derived.by(() =>
{
    if (sections)
    {
        return sections;
    }
    const ordered = [...actions].sort((a, b) => Number(a.style === "destructive") - Number(b.style === "destructive"));
    const result: ContextMenuSection[] = [];
    for (const [index, action] of ordered.entries())
    {
        const previous = ordered[index - 1];
        if (
            !previous || action.group?.id !== previous.group?.id
            || (action.style === "destructive") !== (previous.style === "destructive")
        )
        {
            result.push({ items: [] });
        }
        result[result.length - 1]!.items.push(action);
    }
    return result;
});

function _activate(action: Action): void
{
    if (!open || !action.enabled)
    {
        return;
    }
    onInvoke(action.id);
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
                    aria-label={heading ?? localization.text({ message: "actions-label" })}
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
                            !open && !confirmationPending() && document.hasFocus() && document.visibilityState === "visible"
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
                            {#each displayedSections as section, sectionIndex (section.id ? `group:${section.id}` : `item:${section.items[0]?.id ?? sectionIndex}`)}
                                {#if sectionIndex > 0}<Menu.Separator class="popup-separator" />{/if}
                                <Menu.Group>
                                    {#if section.title}<Menu.GroupHeading class="popup-heading">{
                                            section.title
                                        }</Menu.GroupHeading>{/if}
                                    {#each section.items as action (action.id)}
                                        {@const keys = shortcuts[action.id]}
                                        {#if action.checked !== undefined}
                                            <Menu.CheckboxItem
                                                class="popup-item"
                                                checked={action.checked}
                                                disabled={!action.enabled}
                                                closeOnSelect={false}
                                                onSelect={() => _activate(action)}
                                            >
                                                <span>{text(action.title)}</span>
                                                {#if action.checked}<CheckIcon size={16} />{/if}
                                            </Menu.CheckboxItem>
                                        {:else}
                                            <Menu.Item
                                                class="popup-item"
                                                disabled={!action.enabled}
                                                data-destructive={action.style === "destructive" ? "" : undefined}
                                                closeOnSelect={false}
                                                onSelect={() => _activate(action)}
                                            >
                                                <span>{text(action.title)}</span>
                                                {#if keys}<ShortcutKeys {keys} />{/if}
                                            </Menu.Item>
                                        {/if}
                                    {/each}
                                </Menu.Group>
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
