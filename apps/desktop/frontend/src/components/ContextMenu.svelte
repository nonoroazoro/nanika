<script lang="ts">
import { ContextMenu as Menu } from "bits-ui";
import { uiActivity } from "../ui/activity";
import type { Action } from "../types/Action";
import ShortcutKeys from "./ShortcutKeys.svelte";

const { actions, position, open, heading, shortcuts = {}, onClose, onClosed, onInvoke }: {
    actions: Action[];
    position: [number, number] | null;
    open: boolean;
    heading?: string;
    shortcuts?: Record<string, string[]>;
    onClose: () => void;
    onClosed: () => void;
    onInvoke: (id: string, confirmed: boolean) => void;
} = $props();
let confirmation = $state.raw<Action | null>(null);
let outside = false;
const anchor = $derived({
    getBoundingClientRect: () => new DOMRect(position?.[0] ?? 16, position?.[1] ?? 16, 0, 0)
});
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
    if (event.key === "F5")
    {
        event.preventDefault();
    }
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
        <Menu.Portal>
            <Menu.Content
                id="context-menu"
                class="popup-surface context-menu"
                aria-label={heading ?? "Actions"}
                customAnchor={anchor}
                side="bottom"
                align="start"
                sideOffset={4}
                collisionPadding={8}
                trapFocus={false}
                preventScroll={false}
                onOpenAutoFocus={() =>
                {
                    outside = false;
                }}
                onCloseAutoFocus={event =>
                {
                    if (outside || !document.hasFocus() || document.visibilityState !== "visible")
                    {
                        event.preventDefault();
                    }
                }}
                onInteractOutside={() =>
                {
                    outside = true;
                }}
                onkeydown={_keydown}
                oncontextmenu={event => event.preventDefault()}
                style="width: 16rem; max-height: min(var(--bits-context-menu-content-available-height), calc(100vh - 16px));"
            >
                <Menu.Group>
                    {#if heading}<Menu.GroupHeading class="popup-heading">{heading}</Menu.GroupHeading>{/if}
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
            </Menu.Content>
        </Menu.Portal>
    {/if}
</Menu.Root>
