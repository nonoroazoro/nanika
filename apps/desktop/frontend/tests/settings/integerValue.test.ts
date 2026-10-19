import assert from "node:assert/strict";
import { test } from "vitest";

import { configurationSchema } from "../../fixtures/configuration";
import { integerError, normalizeSettings } from "../../src/settings/integerValue.ts";

import type { ConfigurationSchema } from "../../src/generated/ConfigurationSchema";

const schema: ConfigurationSchema = configurationSchema({
    type: "integer",
    minimum: 1,
    maximum: 5000,
    properties: {},
    required: []
});

test("integer drafts retain invalid input without clamping or coercing it to zero", () =>
{
    for (const value of ["", "0", "-1", "5001", "1.5", "1e3", "123456789012345678901234567890"])
    {
        assert.notEqual(integerError(schema, value), null);
        assert.equal(normalizeSettings({ limit: schema }, { limit: value }).limit, value);
    }
});

test("valid integer drafts normalize at the commit boundary", () =>
{
    for (const value of ["1", "50", "5000"])
    {
        assert.equal(integerError(schema, value), null);
        assert.equal(normalizeSettings({ limit: schema }, { limit: value }).limit, Number(value));
    }
});

test("integer limits preserve their finite value and normalize the whole object", () =>
{
    const limit = { ...schema, type: "integerLimit" } as const;
    for (const unlimited of [false, true])
    {
        assert.deepEqual(normalizeSettings({ limit }, { limit: { unlimited, value: "0050" } }), {
            limit: { unlimited, value: 50 }
        });
    }
    assert.notEqual(integerError(schema, null), null);
    assert.notEqual(integerError({ ...schema, multipleOf: 5 }, "12"), null);
});

test("nested integer configuration preserves string properties and optional fields", () =>
{
    const properties: Record<string, ConfigurationSchema> = {
        entries: configurationSchema({
            type: "array",
            properties: {},
            required: [],
            items: configurationSchema({
                type: "object",
                properties: {
                    limit: schema,
                    name: configurationSchema({ type: "string", properties: {}, required: [] })
                },
                required: []
            })
        })
    };
    assert.deepEqual(normalizeSettings(properties, { entries: [{ limit: "50", name: "001" }, { limit: "1" }] }), {
        entries: [{ limit: 50, name: "001" }, { limit: 1 }]
    });
});
