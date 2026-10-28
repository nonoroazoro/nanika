import assert from "node:assert/strict";
import { expect, test } from "vitest";

import { matchActionShortcut, resolveActionShortcut } from "../../src/logic/action-shortcuts";

import type { Action } from "../../src/generated/Action";

const actions: [Action, Action, Action] = [
    {
        id: "custom.reveal",
        title: "Reveal",
        enabled: true,
        style: "secondary",
        group: null,
        allow_default_execution: false,
        shortcut: { key: { kind: "virtual", value: "Enter" }, modifiers: ["primary"] }
    },
    {
        id: "custom.inspect",
        title: "Inspect",
        enabled: true,
        style: "secondary",
        group: null,
        allow_default_execution: false,
        shortcut: { key: { kind: "virtual", value: "K" }, modifiers: ["shift", "primary"] }
    },
    {
        id: "custom.copy",
        title: "Copy",
        enabled: false,
        style: "secondary",
        group: null,
        allow_default_execution: false,
        shortcut: { key: { kind: "virtual", value: "1" }, modifiers: ["primary", "alt"] }
    }
];
const enter: Parameters<typeof matchActionShortcut>[0] = {
    key: "Enter",
    code: "Enter",
    keyCode: 13,
    metaKey: true,
    ctrlKey: false,
    altKey: false,
    shiftKey: false,
    defaultPrevented: false,
    repeat: false,
    isComposing: false,
    getModifierState: () => false
};

test("matches extension-defined actions only on the current item and platform", () =>
{
    expect(matchActionShortcut(enter, actions, "macos")?.id).toBe("custom.reveal");
    expect(matchActionShortcut({ ...enter, metaKey: false, ctrlKey: true }, actions, "windows")?.id).toBe(
        "custom.reveal"
    );
    expect(matchActionShortcut(enter, actions, "windows")).toBeUndefined();
    expect(matchActionShortcut(enter, actions, null)).toBeUndefined();
    expect(matchActionShortcut(enter, [], "macos")).toBeUndefined();
    expect(matchActionShortcut({ ...enter, key: "k", code: "KeyK", keyCode: 75, shiftKey: true }, actions, "macos")?.id)
        .toBe(
            "custom.inspect"
        );
    expect(matchActionShortcut({ ...enter, key: "1", code: "Digit1", keyCode: 49, altKey: true }, actions, "macos"))
        .toBeUndefined();
    expect(matchActionShortcut({ ...enter, code: "NumpadEnter" }, actions, "macos")?.id).toBe("custom.reveal");
});

test.each([
    { metaKey: false },
    { ctrlKey: true },
    { shiftKey: true },
    { altKey: true },
    { repeat: true },
    { isComposing: true },
    { keyCode: 229 },
    { defaultPrevented: true },
    { key: "Dead" },
    { getModifierState: () => true }
])("preserves input ownership: %o", patch =>
{
    expect(matchActionShortcut({ ...enter, ...patch }, actions, "macos")).toBeUndefined();
});

test("shortcut matching preserves confirmation policy and menu labels use the same declaration", () =>
{
    const confirmed = { ...actions[0], confirmation_title: "Reveal?" };
    expect(matchActionShortcut(enter, [confirmed], "macos")).toBe(confirmed);
    assert.ok(actions[0].shortcut && actions[1].shortcut && actions[2].shortcut);
    expect(resolveActionShortcut(actions[0].shortcut, "macos").keys).toEqual(["⌘", "↵"]);
    expect(resolveActionShortcut(actions[1].shortcut, "windows").keys).toEqual(["Ctrl", "Shift", "K"]);
    expect(resolveActionShortcut(actions[2].shortcut, "macos").keys).toEqual(["⌘", "⌥", "1"]);
});

test("virtual and physical bindings keep their identity when layout changes", () =>
{
    const physical: Action = {
        ...actions[0],
        shortcut: { key: { kind: "physical", value: "KeyI" }, modifiers: ["primary"] }
    };
    const virtual: Action = {
        ...actions[0],
        shortcut: { key: { kind: "virtual", value: "I" }, modifiers: ["primary"] }
    };
    const dvorak = { ...enter, key: "c", code: "KeyI", keyCode: 67 };
    expect(matchActionShortcut(dvorak, [physical], "macos")).toBe(physical);
    expect(matchActionShortcut(dvorak, [virtual], "macos")).toBeUndefined();
    assert.ok(physical.shortcut && virtual.shortcut);
    const resolved = resolveActionShortcut(physical.shortcut, "macos", { KeyI: "C" });
    expect(resolved.keys).toEqual(["⌘", "C"]);
    expect(resolved.matches(dvorak)).toBe(true);
    expect(resolveActionShortcut(physical.shortcut, "macos").keys).toEqual(["⌘", "[KeyI]"]);
    expect(resolveActionShortcut(physical.shortcut, "macos", { KeyI: "I" }).keys).toEqual(["⌘", "I"]);
    expect(resolveActionShortcut(virtual.shortcut, "macos", { KeyI: "C" }).keys).toEqual(["⌘", "I"]);
});

test("extensions may deliberately bind native editing chords", () =>
{
    const copy: Action = {
        ...actions[0],
        shortcut: { key: { kind: "virtual", value: "C" }, modifiers: ["primary"] }
    };
    expect(matchActionShortcut({ ...enter, key: "c", code: "KeyI", keyCode: 67 }, [copy], "macos")).toBe(copy);
});

test("physical numpad Enter remains distinct in matching and display", () =>
{
    const resolved = resolveActionShortcut({
        key: { kind: "physical", value: "NumpadEnter" },
        modifiers: ["primary"]
    }, "windows");
    expect(resolved.keys).toEqual(["Ctrl", "Num ↵"]);
    expect(resolved.matches(enter)).toBe(false);
    expect(resolved.matches({ ...enter, code: "NumpadEnter" })).toBe(true);
});

test.each(["macos", "windows"] as const)("virtual digit labels name the virtual key on %s", platform =>
{
    const shortcut = { key: { kind: "virtual", value: "1" }, modifiers: ["primary"] } as const;
    const resolved = resolveActionShortcut({ ...shortcut, modifiers: [...shortcut.modifiers] }, platform, {
        Digit1: "&"
    });
    expect(resolved.keys).toEqual([platform === "macos" ? "⌘" : "Ctrl", "1"]);
    expect(resolved.matches({ keyCode: 49, code: "Digit1", altKey: false, shiftKey: false })).toBe(true);
});

test("Dvorak physical punctuation cannot relabel a virtual letter", () =>
{
    const layout = { KeyD: "E", KeyE: "." };
    const virtual = resolveActionShortcut(
        {
            key: { kind: "virtual", value: "E" },
            modifiers: ["primary"]
        },
        "macos",
        layout
    );
    const physical = resolveActionShortcut(
        {
            key: { kind: "physical", value: "KeyE" },
            modifiers: ["primary"]
        },
        "macos",
        layout
    );
    const letter = { ...enter, code: "KeyD", key: "e", keyCode: 69 };
    const punctuation = { ...enter, code: "KeyE", key: ".", keyCode: 190 };
    expect(virtual.keys).toEqual(["⌘", "E"]);
    expect(virtual.matches(letter)).toBe(true);
    expect(virtual.matches(punctuation)).toBe(false);
    expect(physical.keys).toEqual(["⌘", "."]);
    expect(physical.matches(letter)).toBe(false);
    expect(physical.matches(punctuation)).toBe(true);
});
