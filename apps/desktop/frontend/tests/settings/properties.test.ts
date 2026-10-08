import { expect, test } from "vitest";

import { configurationProperty } from "./schema";
import { orderedProperties } from "../../src/settings/properties.ts";

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
