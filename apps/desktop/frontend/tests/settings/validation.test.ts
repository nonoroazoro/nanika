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
    assert.deepEqual(settingsError({ limit: integer }, { limit: "" }), { code: "wholeNumber", path: ["limit"] });
    assert.deepEqual(settingsError({ name: text }, { name: "你好" }), {
        code: "textBytes",
        constraint: 4,
        path: ["name"]
    });
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
    assert.deepEqual(settingsError({ entries }, { entries: values }), {
        code: "integerMinimum",
        constraint: 1,
        path: ["entries", 24]
    });
});

test("optional object fields validate only when included and required fields remain mandatory", () =>
{
    const options: ConfigurationSchema = configurationSchema({
        type: "object",
        properties: { limit: integer, label: text },
        required: ["limit"]
    });
    assert.deepEqual(settingsError({ options }, { options: { limit: 1 } }), null);
    assert.deepEqual(settingsError({ options }, { options: {} }), { code: "wholeNumber", path: ["options", "limit"] });
    assert.deepEqual(
        settingsError({ options }, { options: { limit: 1, label: "12345" } }),
        { code: "textBytes", constraint: 4, path: ["options", "label"] }
    );
});

test("optional prototype-named properties are absent until explicitly included", () =>
{
    const schema: ConfigurationSchema = configurationSchema({
        type: "object",
        properties: { constructor: integer, toString: text },
        required: []
    });
    assert.deepEqual(settingsError({ options: schema }, { options: {} }), null);
    assert.deepEqual(
        settingsError({ options: schema }, { options: { constructor: 0 } }),
        { code: "integerMinimum", constraint: 1, path: ["options", "constructor"] }
    );
});

test("integer limits validate remembered numbers even while unlimited", () =>
{
    const limit = configurationSchema({ type: "integerLimit", minimum: 1, maximum: 5000 });
    for (const unlimited of [false, true])
    {
        assert.deepEqual(settingsError({ limit }, { limit: { unlimited, value: "200" } }), null);
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
    assert.deepEqual(settingsError({ limit: unbounded }, { limit: { unlimited: false, value: -50 } }), null);
    assert.notEqual(
        settingsError({ limit: unbounded }, { limit: { unlimited: true, value: Number.MAX_SAFE_INTEGER + 1 } }),
        null
    );
});
