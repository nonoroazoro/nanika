import assert from "node:assert/strict";
import test from "node:test";

import { stringError } from "../../src/settings/stringValue.ts";

test("string length follows the UTF-8 byte contract", () =>
{
    const schema = { type: "string", maxLength: 5 };
    assert.equal(stringError(schema, "hello"), null);
    assert.equal(stringError(schema, "你好"), "Enter at most 5 UTF-8 bytes.");
    assert.equal(stringError({ ...schema, maxLength: 6 }, "你好"), null);
});
