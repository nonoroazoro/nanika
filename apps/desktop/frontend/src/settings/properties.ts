import type { ConfigurationProperty } from "../generated/ConfigurationProperty";

/**
 * Order the declarative Settings fields without depending on JSON object insertion order.
 *
 * @param properties The visible contribution's {@link ConfigurationProperty} values.
 */
export function orderedProperties(
    properties: Record<string, ConfigurationProperty>
): Array<[string, ConfigurationProperty]>
{
    return Object.entries(properties).sort(_compareProperties);
}

/**
 * Group ordered fields by their declared title, retaining first appearance order.
 * @param properties The platform-filtered contribution properties.
 * @param title The contribution's default group title.
 */
export function propertyGroups(
    properties: Record<string, ConfigurationProperty>,
    title: string
): Array<[string, Array<[string, ConfigurationProperty]>]>
{
    const groups = new Map<string, Array<[string, ConfigurationProperty]>>();
    for (const entry of orderedProperties(properties))
    {
        const group = entry[1].group ?? title;
        const entries = groups.get(group) ?? [];
        entries.push(entry);
        groups.set(group, entries);
    }
    return [...groups];
}

function _compareProperties(
    [leftKey, left]: [string, ConfigurationProperty],
    [rightKey, right]: [string, ConfigurationProperty]
): number
{
    const order = (left.order ?? 0) - (right.order ?? 0);
    if (order !== 0 || leftKey === rightKey)
    {
        return order;
    }
    return leftKey < rightKey ? -1 : 1;
}
