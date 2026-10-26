import { expect, test, vi } from "vitest";

import cases from "../../../../../engine/extension-protocol/tests/fixtures/translated-text.json";
import { formatTranslatedText } from "../../src/localization/translated-text";

test("domain names use supplied translations, English fallback, then source fallback", () =>
{
    const text = { fallback: "Source", translations: { en: "Music", "zh-Hans": "音乐" } };
    expect(formatTranslatedText(text, "zh-Hans", 128)).toBe("音乐");
    expect(formatTranslatedText(text, "ja", 128)).toBe("Music");
    expect(formatTranslatedText({ ...text, translations: { "zh-Hans": "音乐" } }, "ja", 128)).toBe("Source");
});

test("all domain translations are bounded and invalid inactive values fail before selection", () =>
{
    for (
        const translations of [{}, { en: "Name", ja: "\u0000" }, { en: "Name", ja: "" }, {
            en: "Name",
            ja: "界".repeat(129)
        }, { en: "Name", "zh_Hans": "音乐" }]
    )
    {
        expect(() => formatTranslatedText({ fallback: "Name", translations }, "en", 128)).toThrow();
    }
    expect(() => formatTranslatedText({ fallback: "Name", translations: { en: "Name" }, message: "other" }, "en", 128))
        .toThrow();
});

test("matching language variants precede English and foreign fallback content stays visible", () =>
{
    const text = { fallback: "音楽箱", translations: { "en-GB": "Music", "zh-TW": "網易雲音樂", ja: "音楽箱" } };
    expect(formatTranslatedText(text, "zh-Hans", 128)).toBe("網易雲音樂");
    expect(formatTranslatedText(text, "en", 128)).toBe("Music");
    expect(formatTranslatedText(text, "ja", 128)).toBe("音楽箱");
    expect(formatTranslatedText(text, "fr", 128)).toBe("Music");
    const both = { ...text, translations: { ...text.translations, "zh-CN": "网易云音乐" } };
    expect(formatTranslatedText(both, "zh-Hans", 128)).toBe("网易云音乐");
    expect(formatTranslatedText(both, "zh-Hant", 128)).toBe("網易雲音樂");
    const japanese = { fallback: "音楽箱", translations: { ja: "音楽箱" } };
    expect(formatTranslatedText(japanese, "en", 128)).toBe("音楽箱");
    expect(formatTranslatedText(japanese, "zh-Hans", 128)).toBe("音楽箱");
    const many = Object.fromEntries(
        Array.from({ length: 64 }, (_, index) => [`en-var${String(index + 1).padStart(4, "0")}`, `Name ${index}`])
    );
    expect(formatTranslatedText({ fallback: "Name", translations: many }, "en", 128)).toBe("Name 0");
});

for (const entry of cases)
{
    test(`shared language matching: ${entry.name}`, () =>
    {
        if (entry.expected === null)
        {
            expect(() => formatTranslatedText(entry.text, entry.locale, 128)).toThrow();
        }
        else
        {
            expect(formatTranslatedText(entry.text, entry.locale, 128)).toBe(entry.expected);
        }
    });
}

test("domain matching is independent of WebView likely-subtags data", () =>
{
    const maximize = vi.spyOn(Intl.Locale.prototype, "maximize").mockImplementation(() =>
    {
        throw new Error("WebView data must not determine domain matching");
    });
    try
    {
        const text = {
            fallback: "Source",
            translations: { "zh-US": "American Chinese", "zh-TW": "Traditional", "en-GB": "English" }
        };
        expect(formatTranslatedText(text, "zh-Hans", 128)).toBe("Traditional");
        expect(formatTranslatedText(text, "en", 128)).toBe("English");
        expect(maximize).not.toHaveBeenCalled();
    }
    finally
    {
        maximize.mockRestore();
    }
});
