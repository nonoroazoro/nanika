import { readFile } from "node:fs/promises";
import { describe, expect, it } from "vitest";

import { LocaleBundle } from "../../src/localization/LocaleBundle";

import type { ConformanceFixture } from "./ConformanceFixture";
import type { LocalizedText } from "../../src/generated/LocalizedText";

const fixtures = new URL("../../../../../engine/localization/tests/fixtures/", import.meta.url);
const fixture = JSON.parse(await readFile(new URL("conformance.json", fixtures), "utf8")) as ConformanceFixture;
const bundles = {
    en: new LocaleBundle("en", [await readFile(new URL("en.ftl", fixtures), "utf8")], fixture.contracts),
    "zh-Hans": new LocaleBundle(
        "zh-Hans",
        [await readFile(new URL("zh-Hans.ftl", fixtures), "utf8")],
        fixture.contracts
    )
};

describe("LocaleBundle", () =>
{
    it.each(fixture.cases)(
        "matches Rust output for $locale / $platform / $text",
        ({ locale, platform, text, expected }) =>
        {
            expect(bundles[locale].format(text, platform, 2048)).toBe(expected);
        }
    );

    it("rejects malformed dynamic references without exposing argument values", () =>
    {
        const invalid: unknown[] = [
            { message: "unknown" },
            { message: "integer" },
            { message: "brand", args: { unexpected: "private content" } },
            { message: "integer", args: { count: Number.MAX_SAFE_INTEGER + 1 } },
            { message: "integer", args: { count: NaN } },
            { message: "integer", args: { count: Infinity } },
            { message: "integer", args: { count: 1.5 } },
            { message: "content", args: { value: {} } },
            { message: "content", args: { value: "x".repeat(2049) } },
            { message: "platform", args: { platform: "windows" } }
        ];
        for (const text of invalid)
        {
            expect(() => bundles.en.format(text as LocalizedText, "macos", 2048)).toThrow();
        }
        expect(() => bundles.en.format({ message: "brand", args: { unexpected: "private content" } }, "macos", 2048))
            .not.toThrow(/private content/u);
    });

    it("enforces destination limits in Unicode scalars and keeps literal content unchanged", () =>
    {
        expect(bundles.en.format("😀", "macos", 1)).toBe("😀");
        expect(() => bundles.en.format("😀😀", "macos", 1)).toThrow();
        expect(() => bundles.en.format({ message: "brand" }, "macos", 2)).toThrow();
        expect(() => bundles.en.format("", "macos", 128)).toThrow();
        expect(() => bundles.en.format("\u0085", "macos", 128)).toThrow();
        expect(bundles.en.format("", "macos", 128, true)).toBe("");
        expect(bundles.en.format("<b>user content</b>", "macos", 128)).toBe("<b>user content</b>");
    });

    it("rejects missing declared messages and duplicate resources before publication", () =>
    {
        expect(() => new LocaleBundle("en", ["hello = Hello"], { missing: { arguments: [] } })).toThrow();
        expect(() => new LocaleBundle("en", ["hello = Hello", "hello = Again"], { hello: { arguments: [] } }))
            .toThrow();
    });
});
