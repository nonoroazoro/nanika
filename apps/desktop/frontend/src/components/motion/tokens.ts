/**
 * Read the existing CSS duration scale without introducing a second token source.
 * @param element Motion owner
 * @param name CSS custom property
 */
export function duration(element: Element, name: string): number
{
    const value = getComputedStyle(element).getPropertyValue(name).trim();
    return Number.parseFloat(value) * (value.endsWith("ms") ? 1 : 1000);
}

/**
 * Preserve CSS cubic-bezier syntax so motion works on the supported WebViews.
 * @param element Motion owner
 */
export function easing(element: Element): string
{
    return getComputedStyle(element).getPropertyValue("--motion-ease").trim();
}
