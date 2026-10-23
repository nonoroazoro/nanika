import type { ConfigurationMenu } from "../generated/ConfigurationMenu";
import type { ConfigurationMenuOperation } from "../generated/ConfigurationMenuOperation";
import type { LocalizedText } from "../generated/LocalizedText";
import type { ContextMenuSection } from "../types/ContextMenuSection";

/**
 * Project host-authorized configuration shortcuts into the shared menu renderer.
 */
export function configurationMenuSections(
    menu: ConfigurationMenu,
    pending: boolean,
    text: (value: LocalizedText) => string
): ContextMenuSection[]
{
    return menu.groups.map(group => ({
        id: group.id,
        title: text(group.title),
        items: group.items.map(item =>
        {
            if (item.kind === "action")
            {
                return { ...item.action, id: `action:${item.action.id}`, enabled: item.action.enabled && !pending };
            }
            return {
                id: `toggle:${item.key}`,
                title: text(item.title),
                checked: item.checked,
                enabled: !pending,
                allow_default_execution: false,
                style: "secondary",
                group: null
            };
        })
    }));
}

/**
 * Resolve only entries present in the menu snapshot, preserving the value revision shown to the user.
 */
export function configurationMenuOperation(
    menu: ConfigurationMenu,
    id: string,
    confirmed: boolean
): ConfigurationMenuOperation | null
{
    for (const group of menu.groups)
    {
        for (const item of group.items)
        {
            if (item.kind === "toggle" && id === `toggle:${item.key}`)
            {
                return { kind: "set", key: item.key, value: !item.checked, revision: menu.revision };
            }
            if (item.kind === "action" && id === `action:${item.action.id}` && item.action.enabled)
            {
                return { kind: "action", id: item.action.id, confirmed };
            }
        }
    }
    return null;
}
