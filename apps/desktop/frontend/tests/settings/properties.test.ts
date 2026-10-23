import { expect, test } from "vitest";

import { configurationProperty } from "../../fixtures/configuration";
import { orderedProperties, propertyGroups } from "../../src/settings/properties.ts";

import type { ConfigurationProperty } from "../../src/generated/ConfigurationProperty";

test("explicit field order is independent of serialized property insertion order", () =>
{
    const boolean: ConfigurationProperty = configurationProperty({
        type: "boolean",
        persistence: "beforeApply",
        title: "Enabled",
        description: null,
        default: true,
        properties: {},
        required: []
    });
    const properties = {
        folders: { ...boolean, order: 100 },
        user: { ...boolean, order: 20 },
        system: { ...boolean, order: 10 },
        beta: boolean,
        alpha: boolean
    };
    const before = structuredClone(properties);
    expect(orderedProperties(properties).map(([key]) => key)).toEqual(["alpha", "beta", "system", "user", "folders"]);
    expect(properties).toEqual(before);
    expect(orderedProperties(Object.fromEntries(Object.entries(properties).reverse()))).toEqual(
        orderedProperties(properties)
    );
});

test("equal group titles remain distinct and translated labels retain group and field identities", () =>
{
    const field = configurationProperty({
        title: "Field",
        default: true,
        type: "boolean",
        persistence: "beforeApply"
    });
    const properties = {
        a: { ...field, group: { id: "first", title: "Options" } },
        b: { ...field, group: { id: "second", title: "Options" } },
        c: { ...field, group: { id: "first", title: "Options" } },
        d: field
    };
    const before = propertyGroups(properties, "Options");
    expect(before.map(([group, items]) => [group.id, items.map(([key]) => key)])).toEqual([
        ["first", ["a", "c"]],
        ["second", ["b"]],
        ["$configuration", ["d"]]
    ]);
    const translated = Object.fromEntries(
        Object.entries(properties).map(([key, value]) => [key, {
            ...value,
            group: value.group ? { ...value.group, title: "选项" } : null
        }])
    );
    expect(propertyGroups(translated, "选项").map(([group, items]) => [group.id, items.map(([key]) => key)]))
        .toEqual(before.map(([group, items]) => [group.id, items.map(([key]) => key)]));
});
