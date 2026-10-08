import assert from "node:assert/strict";
import { test } from "vitest";

import { configurationSchema } from "./schema";
import { settingsError } from "../../src/settings/validation.ts";

import type { ConfigurationSchema } from "../../src/generated/ConfigurationSchema";

const integer: ConfigurationSchema = configurationSchema({
    type: "integer",
    minimum: 1,
    allowUnlimited: true,
    properties: {},
    required: []
});
const text: ConfigurationSchema = configurationSchema({ type: "string", maxLength: 4, properties: {}, required: [] });

test("automatic saves reject incomplete numbers and oversized UTF-8 text", () =>
{
    assert.equal(settingsError({ limit: integer }, { limit: "" }), "limit: Enter a whole number.");
    assert.equal(settingsError({ name: text }, { name: "你好" }), "name: Enter at most 4 UTF-8 bytes.");
    assert.equal(settingsError({ limit: integer }, { limit: null }), null);
});

test("automatic saves validate entries beyond the rendered array page", () =>
{
    const entries: ConfigurationSchema = configurationSchema({
        type: "array",
        items: integer,
        maxItems: 30,
        properties: {},
        required: []
    });
    const values = Array.from({ length: 25 }, () => 1);
    values[24] = 0;
    assert.equal(settingsError({ entries }, { entries: values }), "entries: Entry 25: Enter 1 or more.");
});

test("optional object fields validate only when included and required fields remain mandatory", () =>
{
    const options: ConfigurationSchema = configurationSchema({
        type: "object",
        properties: { limit: integer, label: text },
        required: ["limit"]
    });
    assert.equal(settingsError({ options }, { options: { limit: 1 } }), null);
    assert.equal(settingsError({ options }, { options: {} }), "options: limit: Enter a whole number.");
    assert.equal(
        settingsError({ options }, { options: { limit: 1, label: "12345" } }),
        "options: label: Enter at most 4 UTF-8 bytes."
    );
});

test("optional prototype-named properties are absent until explicitly included", () =>
{
    const schema: ConfigurationSchema = configurationSchema({
        type: "object",
        properties: { constructor: integer, toString: text },
        required: []
    });
    assert.equal(settingsError({ options: schema }, { options: {} }), null);
    assert.equal(
        settingsError({ options: schema }, { options: { constructor: 0 } }),
        "options: constructor: Enter 1 or more."
    );
});
