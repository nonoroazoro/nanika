import { describe, expect, test } from "vitest";

import { isCompositionKey } from "../../src/ui/keyboard";

describe("input method key ownership", () =>
{
    test.each([
        { isComposing: true, keyCode: 13 },
        { isComposing: true, keyCode: 27 },
        { isComposing: false, keyCode: 229 }
    ])("keeps an IME key out of App commands: %o", event =>
    {
        expect(isCompositionKey(event)).toBe(true);
    });

    test.each([13, 27, 38, 40])("allows an ordinary key after composition ends: %s", keyCode =>
    {
        expect(isCompositionKey({ isComposing: false, keyCode })).toBe(false);
    });
});
