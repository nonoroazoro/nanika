import { isCompositionKey } from "../ui/keyboard";

import type { Action } from "../generated/Action";
import type { ActionShortcut } from "../generated/ActionShortcut";
import type { TargetPlatform } from "../generated/TargetPlatform";

/**
 * Match an explicit action on the selected root item outside IME composition.
 * Extensions may deliberately override native editing chords.
 *
 * @param event Input from the root search field
 * @param actions The selected item's host-validated actions
 * @param platform Platform reported by the host
 */
export function matchActionShortcut(
    event: Pick<
        KeyboardEvent,
        | "altKey"
        | "code"
        | "ctrlKey"
        | "defaultPrevented"
        | "getModifierState"
        | "isComposing"
        | "key"
        | "keyCode"
        | "metaKey"
        | "repeat"
        | "shiftKey"
    >,
    actions: readonly Action[],
    platform: TargetPlatform | null
): Action | undefined
{
    const primary = platform === "macos"
        ? event.metaKey && !event.ctrlKey
        : platform === "windows" && event.ctrlKey && !event.metaKey;
    if (
        !primary || event.defaultPrevented || event.repeat || isCompositionKey(event)
        || event.getModifierState("AltGraph") || ["Dead", "Process", "Unidentified"].includes(event.key)
    )
    {
        return undefined;
    }
    return actions.find(action =>
    {
        if (!action.enabled || !action.shortcut)
        {
            return false;
        }
        const resolved = resolveActionShortcut(action.shortcut, platform === "macos" ? "macos" : "windows");
        return resolved.matches(event);
    });
}

/**
 * Resolve the key identity used by both matching and root context-menu presentation.
 * Virtual labels name the declared virtual key, not a character inferred from a physical key.
 *
 * @param shortcut Host-validated action shortcut
 * @param platform Platform reported by the host
 * @param layout Native physical-key labels for chords containing the platform's primary modifier
 */
export function resolveActionShortcut(
    shortcut: ActionShortcut,
    platform: TargetPlatform,
    layout: Readonly<Record<string, string>> = {}
)
{
    const { key, modifiers } = shortcut;
    let virtualCode: number | null = null;
    if (key.kind === "virtual")
    {
        virtualCode = key.value === "Enter" ? 13 : key.value.charCodeAt(0);
    }
    let label = key.value;
    if (key.kind === "physical")
    {
        label = layout[key.value] ?? `[${key.value}]`;
    }
    if (key.value === "Enter")
    {
        label = "↵";
    }
    if (key.value === "NumpadEnter")
    {
        label = "Num ↵";
    }
    return {
        keys: [
            platform === "macos" ? "⌘" : "Ctrl",
            ...(modifiers.includes("alt") ? [platform === "macos" ? "⌥" : "Alt"] : []),
            ...(modifiers.includes("shift") ? ["Shift"] : []),
            label
        ],
        matches: (event: Pick<KeyboardEvent, "altKey" | "code" | "keyCode" | "shiftKey">) =>
            modifiers.includes("primary")
            // eslint-disable-next-line @typescript-eslint/no-deprecated -- Virtual-key identity must survive Shift/Alt character transformations; event.key is text, event.code is physical position.
            && (virtualCode === null ? event.code === key.value : event.keyCode === virtualCode)
            && modifiers.includes("alt") === event.altKey
            && modifiers.includes("shift") === event.shiftKey
    };
}
