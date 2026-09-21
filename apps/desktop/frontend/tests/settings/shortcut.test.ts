import assert from "node:assert/strict";
import test from "node:test";

import { shortcutFromKey, shortcutKeys } from "../../src/settings/shortcut.ts";

const key = {
    key: "k",
    code: "KeyK",
    ctrlKey: true,
    altKey: false,
    shiftKey: false,
    metaKey: false,
    repeat: false,
    isComposing: false
};

void test("shortcut recording preserves physical keys and every modifier", () =>
{
    assert.equal(shortcutFromKey(key), "Ctrl+KeyK");
    assert.equal(shortcutFromKey({ ...key, altKey: true, shiftKey: true, metaKey: true }), "Ctrl+Alt+Shift+Super+KeyK");
    assert.equal(shortcutFromKey({ ...key, key: " ", code: "Space" }), "Ctrl+Space");
    assert.equal(shortcutFromKey({ ...key, key: "Escape", code: "Escape" }), "Ctrl+Escape");
    assert.deepEqual(shortcutKeys("Ctrl+Alt+Digit1"), ["Ctrl", "Alt", "1"]);
});

void test("typing, modifiers, repeats, IME and unidentified events do not create shortcuts", () =>
{
    for (
        const override of [
            { ctrlKey: false },
            { key: "Control" },
            { repeat: true },
            { isComposing: true },
            { key: "Dead" },
            { code: "Unidentified" },
            { code: "" }
        ]
    )
    {
        assert.equal(shortcutFromKey({ ...key, ...override }), null);
    }
});
