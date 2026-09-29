import type { ConfigurationProperty } from "../types/Settings";

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
