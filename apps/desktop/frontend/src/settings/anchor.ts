import type { SettingsSearchTarget } from "../generated/SettingsSearchTarget";

/**
 * Encode stable page and field identities without interpreting them as CSS selectors.
 *
 * @param page The host page or validated extension identity.
 * @param target The destination supplied by the Rust catalog.
 */
export function settingsAnchor(page: string, target: SettingsSearchTarget): string
{
    const parts = [page, target.kind];
    if ("key" in target)
    {
        parts.push(target.key);
    }
    return `settings/${parts.map(encodeURIComponent).join("/")}`;
}
