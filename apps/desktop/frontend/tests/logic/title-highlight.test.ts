import { describe, expect, it } from "vitest";

import { highlightTitle } from "../../src/logic/title-highlight";

describe("highlightTitle", () =>
{
    it("matches across directional isolates while preserving the original display text", () =>
    {
        for (const [title, query] of [["shortcut\u206842\u2069", "shortcut42"], ["音\u2068乐\u2069", "音乐"]] as const)
        {
            const parts = highlightTitle(title, query);
            expect(parts.map(part => part.text).join("")).toBe(title);
            const marked = parts.filter(part => part.highlighted).map(part => part.text).join("");
            expect(marked.replace(/[\u2066-\u2069]/gu, "")).toBe(query);
            expect(highlightTitle(title, title)).toEqual(parts);
        }
    });

    it("uses the displayed language and never reuses alias or pinyin offsets", () =>
    {
        expect(highlightTitle("Empty Trash", "empty")).toEqual([
            { text: "Empty", highlighted: true },
            { text: " Trash", highlighted: false }
        ]);
        for (const query of ["Empty", "feizhilou", "", " .*"])
        {
            expect(highlightTitle("清倒废纸篓", query)).toEqual([{ text: "清倒废纸篓", highlighted: false }]);
        }
        expect(highlightTitle("清倒废纸篓", "废纸")).toEqual([
            { text: "清倒", highlighted: false },
            { text: "废纸", highlighted: true },
            { text: "篓", highlighted: false }
        ]);
    });

    it("preserves original graphemes, combining marks and surrogate pairs", () =>
    {
        const title = "👩🏽‍💻 Cafe\u0301 𠮷野家";
        const parts = highlightTitle(title, "café 𠮷");
        expect(parts).toEqual([
            { text: "👩🏽‍💻 ", highlighted: false },
            { text: "Cafe\u0301", highlighted: true },
            { text: " ", highlighted: false },
            { text: "𠮷", highlighted: true },
            { text: "野家", highlighted: false }
        ]);
        expect(parts.map(part => part.text).join("")).toBe(title);
        expect(highlightTitle("x\u0301", "x")).toEqual([{ text: "x\u0301", highlighted: true }]);
    });

    it("bounds decorative work on oversized titles while retaining every character", () =>
    {
        const title = `${" ".repeat(1023)}𠮷${"x".repeat(60_000)}𠮷`;
        const parts = highlightTitle(title, "𠮷");
        expect(parts.filter(part => part.highlighted)).toEqual([{ text: "𠮷", highlighted: true }]);
        expect(parts.map(part => part.text).join("")).toBe(title);
        expect(parts.at(-1)?.text.endsWith("𠮷")).toBe(true);
    });
});
