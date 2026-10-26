import { describe, expect, it } from "vitest";

import { ResourceSet } from "../../src/localization/ResourceSet";

import type { PreparedResources } from "../../src/generated/PreparedResources";

const resources: PreparedResources = {
    identity: "host-computed-resource-digest",
    defaultLocale: "en",
    locales: {
        en: {
            sources: ["-brand = English brand\nhello = Hello, { -brand }\nonly-default = Default"],
            contracts: { hello: { arguments: [] }, "only-default": { arguments: [] } }
        },
        "zh-Hans": {
            sources: ["-brand = 中文品牌\nhello = 你好，{ -brand }"],
            contracts: { hello: { arguments: [] } }
        }
    }
};

describe("ResourceSet", () =>
{
    it("resolves fallback with terms from the same bundle", () =>
    {
        const set = new ResourceSet(resources);
        expect(set.format("zh-Hans", { message: "hello" }, "macos", 128)).toBe("你好，\u2068中文品牌\u2069");
        expect(set.format("zh-Hans", { message: "only-default" }, "macos", 128)).toBe("Default");
        expect(() => set.format("zh-Hans", { message: "unknown" }, "macos", 128)).toThrow();
        expect(() => set.format("zh-Hans", { message: "hello", args: { invalid: "secret" } }, "macos", 128)).toThrow();
    });

    it("supports single-language external resources and retains old instances across replacement", () =>
    {
        const before = new ResourceSet(resources);
        const after = new ResourceSet({
            identity: "new-resource-digest",
            defaultLocale: "zh-Hans",
            locales: {
                "zh-Hans": { sources: ["hello = 新资源"], contracts: { hello: { arguments: [] } } }
            }
        });
        expect(after.format("en", { message: "hello" }, "windows", 128)).toBe("新资源");
        expect(before.format("en", { message: "hello" }, "windows", 128)).toContain("English brand");
        expect(() => new ResourceSet({ ...resources, locales: {} })).toThrow();
    });
});

it("domain translations use the requested UI locale even when package resources fall back", () =>
{
    const set = new ResourceSet({ ...resources, locales: { en: resources.locales.en } });
    const text = { fallback: "NeteaseMusic", translations: { en: "NetEaseMusic", "zh-Hans": "网易云音乐" } };
    expect(set.format("zh-Hans", text, "macos", 128)).toBe("网易云音乐");
});
