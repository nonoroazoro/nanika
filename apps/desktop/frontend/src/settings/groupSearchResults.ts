import type { SettingsSearchEntry } from "../generated/SettingsSearchEntry";
import type { SettingsSearchGroup } from "../types/SettingsSearchGroup";

/**
 * Group matched entries without changing the host's settings presentation order.
 * The heading already provides the page destination.
 *
 * @param results The matched entries in page order supplied by the host.
 */
export function groupSearchResults(results: readonly SettingsSearchEntry[]): SettingsSearchGroup[]
{
    const groups = new Map<string, SettingsSearchGroup>();
    for (const entry of results)
    {
        let group = groups.get(entry.pageId);
        if (!group)
        {
            group = {
                pageId: entry.pageId,
                title: entry.pageTitle,
                entries: [],
                ...(entry.pageResourceScope ? { resourceScope: entry.pageResourceScope } : {})
            };
            groups.set(entry.pageId, group);
        }
        if (entry.target.kind !== "page")
        {
            group.entries.push(entry);
        }
    }
    return [...groups.values()];
}
