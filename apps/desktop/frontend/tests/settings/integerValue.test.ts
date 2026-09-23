import assert from "node:assert/strict";
import { test } from "vitest";

import { integerError, normalizeSettings } from "../../src/settings/integerValue.ts";

import type { ConfigurationSchema } from "../../src/types/Settings.ts";

const schema: ConfigurationSchema = {
    type: "integer",
    minimum: 1,
    maximum: 5000,
    allowUnlimited: true,
    properties: {},
    required: []
};

test("integer drafts retain invalid input without clamping or coercing it to zero", () =>
{
    for (const value of ["", "0", "-1", "5001", "1.5", "1e3", "123456789012345678901234567890"])
    {
        assert.notEqual(integerError(schema, value), null);
        assert.equal(normalizeSettings({ limit: schema }, { limit: value }).limit, value);
    }
});

test("valid integer drafts normalize only at the persistence boundary", () =>
{
    for (const value of ["1", "50", "5000"])
    {
        assert.equal(integerError(schema, value), null);
        assert.equal(normalizeSettings({ limit: schema }, { limit: value }).limit, Number(value));
    }
});

test("unlimited is explicit and requires schema support", () =>
{
    assert.equal(integerError(schema, null), null);
    assert.notEqual(integerError({ ...schema, allowUnlimited: false }, null), null);
    assert.equal(normalizeSettings({ limit: schema }, { limit: null }).limit, null);
    assert.notEqual(integerError({ ...schema, multipleOf: 5 }, "12"), null);
});

test("nested integer configuration preserves string properties and optional fields", () =>
{
    const properties: Record<string, ConfigurationSchema> = {
        entries: {
            type: "array",
            properties: {},
            required: [],
            items: {
                type: "object",
                properties: { limit: schema, name: { type: "string", properties: {}, required: [] } },
                required: []
            }
        }
    };
    assert.deepEqual(normalizeSettings(properties, { entries: [{ limit: "50", name: "001" }, { limit: null }] }), {
        entries: [{ limit: 50, name: "001" }, { limit: null }]
    });
});
