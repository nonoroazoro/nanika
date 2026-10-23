import { expect, test } from "vitest";

import { configurationMenuOperation, configurationMenuSections } from "../../src/logic/configuration-menu";

import type { ConfigurationMenu } from "../../src/generated/ConfigurationMenu";
import type { LocalizedText } from "../../src/generated/LocalizedText";

const menu: ConfigurationMenu = {
    revision: 42,
    groups: [
        {
            id: "appearance",
            title: "Appearance",
            items: [{ kind: "toggle", key: "example.colors", title: "Colors", checked: true }]
        },
        {
            id: "danger-zone",
            title: "Danger zone",
            items: [{
                kind: "action",
                action: {
                    id: "example.reset",
                    title: "Reset",
                    confirmation_title: "Reset all?",
                    enabled: true,
                    allow_default_execution: false,
                    style: "destructive",
                    group: null
                }
            }]
        }
    ]
};

test("generic menu references retain group order, check state and destructive confirmation", () =>
{
    const sections = configurationMenuSections(menu, false, _text);
    expect(sections.map(section => section.title)).toEqual(["Appearance", "Danger zone"]);
    expect(sections[0]?.items[0]).toMatchObject({ title: "Colors", checked: true, enabled: true });
    expect(sections[1]?.items[0]).toMatchObject({ style: "destructive", confirmation_title: "Reset all?" });
    expect(configurationMenuOperation(menu, "toggle:example.colors", false)).toEqual({
        kind: "set",
        key: "example.colors",
        value: false,
        revision: 42
    });
    expect(configurationMenuOperation(menu, "action:example.reset", true)).toEqual({
        kind: "action",
        id: "example.reset",
        confirmed: true
    });
    expect(configurationMenuOperation(menu, "toggle:undeclared", false)).toBeNull();
    expect(configurationMenuSections(menu, true, _text).flatMap(section => section.items).every(item => !item.enabled))
        .toBe(
            true
        );
    expect(configurationMenuSections({ revision: 0, groups: [] }, false, _text)).toEqual([]);
});

function _text(value: LocalizedText): string
{
    if (typeof value !== "string")
    {
        throw new Error("Expected a literal fixture");
    }
    return value;
}
