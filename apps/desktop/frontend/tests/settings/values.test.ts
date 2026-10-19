import assert from "node:assert/strict";
import { test } from "vitest";

import { configurationSchema } from "../../fixtures/configuration";
import { emptyValue } from "../../src/settings/values";

test("empty numeric values respect optional lower and upper bounds and step", () =>
{
    for (const type of ["integer", "integerLimit"] as const)
    {
        for (
            const [constraints, expected] of [
                [{}, 0],
                [{ minimum: 1, multipleOf: 2 }, 2],
                [{ maximum: -1, multipleOf: 2 }, -2]
            ] as const
        )
        {
            assert.deepEqual(
                emptyValue(configurationSchema({ type, ...constraints })),
                type === "integerLimit" ? { unlimited: false, value: expected } : expected
            );
        }
    }
});
