import assert from "node:assert/strict";
import { test } from "vitest";

import { configurationSchema } from "../../fixtures/configuration";
import { stringError } from "../../src/settings/stringValue.ts";

import type { ConfigurationSchema } from "../../src/generated/ConfigurationSchema";

test("string length follows the UTF-8 byte contract", () =>
{
    const schema: ConfigurationSchema = configurationSchema({
        type: "string",
        maxLength: 5,
        properties: {},
        required: []
    });
    assert.equal(stringError(schema, "hello"), null);
    assert.deepEqual(stringError(schema, "你好"), { code: "textBytes", constraint: 5, path: [] });
    assert.equal(stringError({ ...schema, maxLength: 6 }, "你好"), null);
});
