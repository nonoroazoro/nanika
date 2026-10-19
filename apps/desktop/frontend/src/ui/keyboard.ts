/**
 * Identifies keystrokes owned by an input method rather than App shortcuts.
 * WebKit can dispatch compositionend before the final keydown; UI Events still marks that key as 229.
 *
 * @param event Keyboard event delivered by the WebView
 */
export function isCompositionKey(event: Pick<KeyboardEvent, "isComposing" | "keyCode">): boolean
{
    // eslint-disable-next-line @typescript-eslint/no-deprecated -- UI Events' IME marker remains necessary when WebKit ends composition before keydown.
    return event.isComposing || event.keyCode === 229;
}
