import assert from "node:assert/strict";
import { test } from "vitest";

import { configurationSchema } from "../../fixtures/configuration";
import { settingsError } from "../../src/settings/validation.ts";

import type { ConfigurationSchema } from "../../src/generated/ConfigurationSchema";
import type { JsonValue } from "../../src/generated/serde_json/JsonValue";

const integer: ConfigurationSchema = configurationSchema({
    type: "integer",
    minimum: 1,
    properties: {},
    required: []
});
const text: ConfigurationSchema = configurationSchema({ type: "string", maxLength: 4, properties: {}, required: [] });

test("automatic saves reject incomplete numbers and oversized UTF-8 text", () =>
{
    assert.equal(settingsError({ limit: integer }, { limit: "" }), "limit: Enter a whole number.");
    assert.equal(settingsError({ name: text }, { name: "你好" }), "name: Enter at most 4 UTF-8 bytes.");
    assert.notEqual(settingsError({ limit: integer }, { limit: null }), null);
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

test("integer limits validate remembered numbers even while unlimited", () =>
{
    const limit = configurationSchema({ type: "integerLimit", minimum: 1, maximum: 5000 });
    for (const unlimited of [false, true])
    {
        assert.equal(settingsError({ limit }, { limit: { unlimited, value: "200" } }), null);
        assert.notEqual(settingsError({ limit }, { limit: { unlimited, value: "" } }), null);
        assert.notEqual(settingsError({ limit }, { limit: { unlimited, value: 5001 } }), null);
    }
    for (
        const value of [null, 0, { unlimited: true }, { unlimited: "true", value: 50 }, {
            unlimited: true,
            value: 50,
            extra: 1
        }] as JsonValue[]
    )
    {
        assert.notEqual(settingsError({ limit }, { limit: value }), null);
    }
    const unbounded = configurationSchema({ type: "integerLimit" });
    assert.equal(settingsError({ limit: unbounded }, { limit: { unlimited: false, value: -50 } }), null);
    assert.notEqual(
        settingsError({ limit: unbounded }, { limit: { unlimited: true, value: Number.MAX_SAFE_INTEGER + 1 } }),
        null
    );
});
