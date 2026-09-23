import assert from "node:assert/strict";
import { test } from "vitest";

import { stringError } from "../../src/settings/stringValue.ts";

import type { ConfigurationSchema } from "../../src/types/Settings.ts";

test("string length follows the UTF-8 byte contract", () =>
{
    const schema: ConfigurationSchema = { type: "string", maxLength: 5, properties: {}, required: [] };
    assert.equal(stringError(schema, "hello"), null);
    assert.equal(stringError(schema, "你好"), "Enter at most 5 UTF-8 bytes.");
    assert.equal(stringError({ ...schema, maxLength: 6 }, "你好"), null);
});
