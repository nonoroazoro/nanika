/**
 * Reveal a Settings field inside its owning viewport. Element.scrollIntoView also scrolls
 * clipping ancestors and can move the whole WebView beneath the native titlebar.
 *
 * @param viewport The right-hand {@link HTMLElement} scroll container.
 * @param target The rendered field inside that container.
 */
export function revealSetting(
    viewport: Pick<HTMLElement, "clientHeight" | "clientTop" | "getBoundingClientRect" | "scrollHeight" | "scrollTop">,
    target: Pick<HTMLElement, "getBoundingClientRect">
): void
{
    const field = target.getBoundingClientRect();
    const bounds = viewport.getBoundingClientRect();
    const top = viewport.scrollTop + field.top - bounds.top - viewport.clientTop;
    const centered = top - Math.max(0, (viewport.clientHeight - field.height) / 2);
    viewport.scrollTop = Math.max(0, Math.min(centered, viewport.scrollHeight - viewport.clientHeight));
}
