import type { SettingsSearchEntry } from "../generated/SettingsSearchEntry";
import type { SettingsSearchGroup } from "../types/SettingsSearchGroup";

/**
 * Keep page groups in navigation order while preserving the host's ranking inside
 * each group. The heading already provides the page destination.
 *
 * @param results The completed ranking supplied by the host.
 * @param pageOrder The same page identities and order used by Settings navigation.
 */
export function groupSearchResults(
    results: readonly SettingsSearchEntry[],
    pageOrder: readonly string[]
): SettingsSearchGroup[]
{
    const groups = new Map<string, SettingsSearchGroup>();
    for (const entry of results)
    {
        let group = groups.get(entry.pageId);
        if (!group)
        {
            group = { pageId: entry.pageId, title: entry.pageTitle, entries: [] };
            groups.set(entry.pageId, group);
        }
        if (entry.target.kind !== "page")
        {
            group.entries.push(entry);
        }
    }
    const positions = new Map(pageOrder.map((pageId, index) => [pageId, index]));
    return [...groups.values()].sort((left, right) =>
    {
        return (positions.get(left.pageId) ?? positions.size) - (positions.get(right.pageId) ?? positions.size);
    });
}
