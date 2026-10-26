import { expect, test, vi } from "vitest";

import { configurationProperty } from "../../fixtures/configuration";
import { ExtensionPresentation } from "../../src/localization/ExtensionPresentation";
import { ResourceSet } from "../../src/localization/ResourceSet";

import type { ConfigurationContribution } from "../../src/generated/ConfigurationContribution";

const scope = { extensionId: "example.extension", identity: "a".repeat(64) };
const name = { message: "extension-name" };

test("name preparation covers both locales and repeated rendering never formats again", () =>
{
    const resources = _resources("应用程序");
    const format = vi.spyOn(resources, "format");
    const prepared = new ExtensionPresentation(scope.extensionId, name, scope, resources, "macos");
    expect(format).toHaveBeenCalledTimes(2);
    for (let index = 0; index < 100; index++)
    {
        const locale = index % 2 ? "en" : "zh-Hans";
        expect(prepared.text({ message: "extension-name" }, locale)).toBe(
            locale === "en" ? "Applications" : "应用程序"
        );
        expect(prepared.matches(scope.extensionId, name, { ...scope }, "macos")).toBe(true);
    }
    expect(format).toHaveBeenCalledTimes(2);
    expect(prepared.matches(scope.extensionId, name, scope, "windows")).toBe(false);
    expect(prepared.matches(scope.extensionId, "Renamed", scope, "macos")).toBe(false);
    expect(prepared.matches(scope.extensionId, name, { ...scope, identity: "b".repeat(64) }, "macos")).toBe(false);
});

test("foreign, absent, or mismatched resources cannot publish a name", () =>
{
    const resources = _resources("应用程序");
    expect(() => new ExtensionPresentation("another.extension", name, scope, resources, "macos")).toThrow("owner");
    expect(() => new ExtensionPresentation(scope.extensionId, name, scope, undefined, "macos")).toThrow("owner");
    expect(() => new ExtensionPresentation(scope.extensionId, name, undefined, resources, "macos")).toThrow("owner");
    expect(() => new ExtensionPresentation(scope.extensionId, name, undefined, undefined, "macos")).toThrow(
        "requires localization resources"
    );
    expect(() => new ExtensionPresentation(scope.extensionId, { message: "unknown" }, scope, resources, "macos"))
        .toThrow();
});

test("a name must fit the original UTF-8 byte and control-character limits in either locale", () =>
{
    for (const value of ["界".repeat(43), "First\n    Second"])
    {
        expect(() => new ExtensionPresentation(scope.extensionId, name, scope, _resources(value), "macos"))
            .toThrow("limits");
    }
    const literal = new ExtensionPresentation(scope.extensionId, "User 应用", undefined, undefined, "macos");
    expect(literal.name("zh-Hans")).toBe("User 应用");
    expect(literal.name("en")).toBe("User 应用");
    expect(literal.text("User content", "zh-Hans")).toBe("User content");
});

test("configuration search labels are prepared once and changing metadata requires a new presentation", () =>
{
    const resources = _resources("应用程序");
    const format = vi.spyOn(resources, "format");
    const configuration: ConfigurationContribution = {
        title: name,
        properties: {
            enabled: configurationProperty({
                title: name,
                description: name,
                type: "boolean",
                default: true,
                persistence: "beforeApply"
            })
        },
        actions: [],
        quickMenu: []
    };
    const prepared = new ExtensionPresentation(
        scope.extensionId,
        "Literal name",
        scope,
        resources,
        "macos",
        configuration
    );
    const calls = format.mock.calls.length;
    for (let index = 0; index < 100; index++)
    {
        expect(prepared.text(name, "zh-Hans")).toBe("应用程序");
        expect(prepared.matches(scope.extensionId, "Literal name", scope, "macos", structuredClone(configuration)))
            .toBe(true);
    }
    expect(format).toHaveBeenCalledTimes(calls);
    expect(prepared.matches(scope.extensionId, "Literal name", scope, "macos", { ...configuration, title: "Changed" }))
        .toBe(false);
    expect(() =>
    {
        return new ExtensionPresentation(scope.extensionId, "Name", scope, resources, "macos", {
            ...configuration,
            title: { message: "unknown" }
        });
    }).toThrow();
    expect(() =>
    {
        return new ExtensionPresentation(scope.extensionId, "Name", scope, resources, "macos", {
            ...configuration,
            title: "界".repeat(43)
        });
    }).toThrow("limits");
});

function _resources(chinese: string): ResourceSet
{
    return new ResourceSet({
        identity: scope.identity,
        defaultLocale: "en",
        locales: {
            en: { sources: ["extension-name = Applications"], contracts: { "extension-name": { arguments: [] } } },
            "zh-Hans": { sources: [`extension-name = ${chinese}`], contracts: { "extension-name": { arguments: [] } } }
        }
    });
}

test("source-owned extension names can switch without package resources", () =>
{
    const title = { fallback: "Music", translations: { en: "Music", "zh-Hans": "音乐" } };
    const prepared = new ExtensionPresentation(scope.extensionId, title, undefined, undefined, "macos");
    expect(prepared.name("en")).toBe("Music");
    expect(prepared.name("zh-Hans")).toBe("音乐");
});
