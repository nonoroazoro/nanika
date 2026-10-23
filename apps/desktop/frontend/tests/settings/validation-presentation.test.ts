import { expect, test } from "vitest";

import { configurationSchema } from "../../fixtures/configuration";
import { hostResources } from "../../src/generated/host-resources";
import { ResourceSet } from "../../src/localization/ResourceSet";
import { settingsErrorText } from "../../src/settings/error-presentation";
import { prepareSetting } from "../../src/settings/validation";
import { validationErrorText } from "../../src/settings/validation-presentation";

import type { ConfigurationSchema } from "../../src/generated/ConfigurationSchema";
import type { JsonValue } from "../../src/generated/serde_json/JsonValue";
import type { SettingsValidationError } from "../../src/settings/SettingsValidationError";

const integer = configurationSchema({ type: "integer", minimum: 3, maximum: 9, multipleOf: 3 });
const cases: Record<
    SettingsValidationError["code"],
    { draft: JsonValue; schema: ConfigurationSchema | null; word: string; }
> = {
    unknownSetting: { schema: null, draft: 3, word: "未知" },
    wholeNumber: { schema: integer, draft: "1.5", word: "整数" },
    integerMinimum: { schema: integer, draft: 0, word: "不小于" },
    integerMaximum: { schema: integer, draft: 12, word: "不大于" },
    integerMultiple: { schema: integer, draft: 4, word: "整数倍" },
    integerLimit: { schema: { ...integer, type: "integerLimit" }, draft: { unlimited: true }, word: "无限制" },
    textRequired: { schema: configurationSchema({ type: "string" }), draft: false, word: "文本" },
    textBytes: { schema: configurationSchema({ type: "string", maxLength: 4 }), draft: "你好", word: "UTF-8" },
    booleanRequired: { schema: configurationSchema({ type: "boolean" }), draft: 1, word: "开启或关闭" },
    arrayRequired: {
        schema: configurationSchema({ type: "array", items: integer, maxItems: 1 }),
        draft: {},
        word: "列表"
    },
    arrayLength: {
        schema: configurationSchema({ type: "array", items: integer, maxItems: 1 }),
        draft: [3, 6],
        word: "条目"
    },
    objectRequired: {
        schema: configurationSchema({ type: "object", properties: { count: integer }, required: ["count"] }),
        draft: [],
        word: "对象"
    }
};

test("every validation code formats in both languages without retaining or changing the draft", () =>
{
    const resources = new ResourceSet(hostResources);
    for (const [code, { schema, draft, word }] of Object.entries(cases))
    {
        const original = structuredClone(draft);
        const result = prepareSetting(schema ? { setting: schema } : {}, "setting", draft);
        expect("error" in result).toBe(true);
        if (!("error" in result) || typeof result.error === "string")
        {
            throw new Error("Expected structured validation result");
        }
        const error = result.error;
        expect(error.code).toBe(code);
        expect(error.path).toEqual(["setting"]);
        expect(error).not.toHaveProperty("draft");
        expect(error).not.toHaveProperty("message");
        for (const platform of ["macos", "windows"] as const)
        {
            const english = validationErrorText(error, value => resources.format("en", value, platform, 2048));
            const chinese = settingsErrorText(
                error,
                "zh-Hans",
                value => resources.format("zh-Hans", value, platform, 2048)
            );
            expect(english).toContain("setting");
            expect(chinese).toContain(word);
            expect(chinese).not.toBe(english);
            if ("constraint" in error)
            {
                expect(chinese).toContain(String(error.constraint));
            }
        }
        expect(draft).toEqual(original);
    }
});

test("nested paths retain literal keys and zero-based indexes while presentation translates their context", () =>
{
    const resources = new ResourceSet(hostResources);
    const error: SettingsValidationError = {
        code: "integerMinimum",
        constraint: 3,
        path: ["plugin.options", 24, "constructor"]
    };
    for (const [locale, entry] of [["en", "Entry"], ["zh-Hans", "个条目"]] as const)
    {
        const text = validationErrorText(error, value => resources.format(locale, value, "macos", 2048));
        expect(text).toContain("plugin.options");
        expect(text).toContain("constructor");
        expect(text).toContain(entry);
        expect(text).toContain("25");
        expect(text).toContain("3");
    }
    expect(error.path).toEqual(["plugin.options", 24, "constructor"]);
    expect(validationErrorText(null, () =>
    {
        throw new Error("Valid input must not format an error");
    })).toBe("");
    const longest: SettingsValidationError = {
        code: "wholeNumber",
        path: Array.from({ length: 5 }, () => "k".repeat(128))
    };
    expect(validationErrorText(longest, value => resources.format("zh-Hans", value, "windows", 2048)).length)
        .toBeLessThan(2048);
});
