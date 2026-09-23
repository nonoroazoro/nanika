export function shortcutKeys(value: string): string[]
{
    return value.split("+").map(key => key.replace(/^Key([A-Z])$/, "$1").replace(/^Digit([0-9])$/, "$1"));
}

export function shortcutModifiers(event: Pick<KeyboardEvent, "altKey" | "ctrlKey" | "metaKey" | "shiftKey">): string[]
{
    return [event.ctrlKey && "Ctrl", event.altKey && "Alt", event.shiftKey && "Shift", event.metaKey && "Super"]
        .filter((key): key is string => typeof key === "string");
}

export function shortcutFromKey(
    event: Pick<
        KeyboardEvent,
        "altKey" | "code" | "ctrlKey" | "isComposing" | "key" | "metaKey" | "repeat" | "shiftKey"
    >
): string | null
{
    if (
        event.repeat || event.isComposing || !event.code || event.code === "Unidentified"
        || ["Control", "Alt", "Shift", "Meta", "AltGraph", "Dead", "Process"].includes(event.key)
    )
    {
        return null;
    }
    if (!event.ctrlKey && !event.altKey && !event.metaKey)
    {
        return null;
    }
    return [...shortcutModifiers(event), event.code].join("+");
}
