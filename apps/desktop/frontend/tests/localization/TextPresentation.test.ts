import { expect, test, vi } from "vitest";

import { actionFields } from "../../src/localization/action-fields";
import { ResourceSet } from "../../src/localization/ResourceSet";
import { TextPresentation } from "../../src/localization/TextPresentation";

import type { Action } from "../../src/generated/Action";

test("all action surfaces retain both translations and switching never formats again", () =>
{
    const resources = _resources("运行");
    const format = vi.spyOn(resources, "format");
    const action: Action = {
        id: "run",
        title: { message: "label" },
        confirmation_title: { message: "label" },
        confirmation_description: { message: "label" },
        group: { id: "group", title: { message: "label" } },
        style: "destructive",
        enabled: true,
        allow_default_execution: false
    };
    const before = structuredClone(action);
    const presentation = new TextPresentation(resources, "macos", actionFields([action]));
    const calls = format.mock.calls.length;
    for (let index = 0; index < 100; index++)
    {
        expect(presentation.text(action.title, "en")).toBe("Run");
        expect(presentation.text(action.title, "zh-Hans")).toBe("运行");
    }
    expect(format).toHaveBeenCalledTimes(calls);
    expect(action).toEqual(before);
    expect(presentation.resources).toBe(resources);
    expect(() => presentation.text({ message: "unprepared" }, "en")).toThrow("not prepared");
});

test("action preparation preserves scalar limits and rejects an invalid inactive locale before publication", () =>
{
    const label = { message: "label" };
    const fields = [{ value: label, maximum: 128 }];
    expect(new TextPresentation(_resources("界".repeat(128)), "macos", fields).text(label, "zh-Hans"))
        .toBe("界".repeat(128));
    expect(() => new TextPresentation(_resources("界".repeat(129)), "macos", fields)).toThrow();
    expect(() => new TextPresentation(undefined, "macos", fields)).toThrow();
    expect(() => new TextPresentation(undefined, "macos", [{ value: "bad\u0000", maximum: 128 }])).toThrow();
    expect(new TextPresentation(undefined, "macos", [{ value: "line\nline", maximum: 128 }]).text("line\nline", "en"))
        .toBe("line\nline");
});

test("typed bytes prepare without package resources and warm switching only reads stored text", async () =>
{
    const bytesModule = await import("../../src/localization/bytes");
    const format = vi.spyOn(bytesModule, "formatBytes");
    const value = { bytes: 1536 };
    const before = structuredClone(value);
    const presentation = new TextPresentation(undefined, "macos", [{ value, maximum: 2048 }]);
    expect(format).toHaveBeenCalledTimes(2);
    for (let index = 0; index < 100; index++)
    {
        expect(presentation.text(value, "en")).toBe("1.5 KiB");
        expect(presentation.text(value, "zh-Hans")).toBe("1.5 KiB");
    }
    expect(format).toHaveBeenCalledTimes(2);
    expect(presentation.has(value)).toBe(true);
    expect(value).toEqual(before);
    expect(() => presentation.text({ bytes: 1537 }, "en")).toThrow("not prepared");
    format.mockRestore();
});

test("typed bytes obey publication bounds and invalid updates cannot replace retained values", () =>
{
    const value = { bytes: 1536 };
    const retained = new TextPresentation(undefined, "macos", [{ value, maximum: 2048 }]);
    expect(() => new TextPresentation(undefined, "macos", [{ value, maximum: 3 }])).toThrow();
    expect(() => new TextPresentation(undefined, "macos", [{ value: { bytes: -1 }, maximum: 2048 }])).toThrow();
    expect(retained.text(value, "zh-Hans")).toBe("1.5 KiB");
});

function _resources(chinese: string): ResourceSet
{
    return new ResourceSet({
        identity: "a".repeat(64),
        defaultLocale: "en",
        locales: {
            en: { sources: ["label = Run"], contracts: { label: { arguments: [] } } },
            "zh-Hans": { sources: [`label = ${chinese}`], contracts: { label: { arguments: [] } } }
        }
    });
}

test("native names switch repeatedly without resources and reject invalid inactive translations", () =>
{
    const value = { fallback: "NeteaseMusic", translations: { en: "NetEaseMusic", "zh-Hans": "网易云音乐" } };
    const before = structuredClone(value);
    const presentation = new TextPresentation(undefined, "macos", [{ value, maximum: 128 }]);
    for (let index = 0; index < 100; index++)
    {
        expect(presentation.text(value, "en")).toBe("NetEaseMusic");
        expect(presentation.text(value, "zh-Hans")).toBe("网易云音乐");
    }
    expect(value).toEqual(before);
    expect(() =>
    {
        return new TextPresentation(undefined, "macos", [{
            value: { ...value, translations: { ...value.translations, ja: "bad\u0000" } },
            maximum: 128
        }]);
    }).toThrow();
    expect(presentation.text(value, "zh-Hans")).toBe("网易云音乐");
});
