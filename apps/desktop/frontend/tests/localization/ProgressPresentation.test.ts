import { expect, test, vi } from "vitest";

import { ProgressPresentation } from "../../src/localization/ProgressPresentation";
import { ResourceSet } from "../../src/localization/ResourceSet";

test("progress retains translations across work-unit and language changes", () =>
{
    const resources = _resources("扫描");
    const format = vi.spyOn(resources, "format");
    const progress = { label: { message: "scan" }, completed: 1, total: 3 };
    const prepared = new ProgressPresentation(progress, resources, "macos");
    const next = new ProgressPresentation({ ...progress, completed: 2 }, resources, "macos", prepared);
    expect(prepared.label("en")).toBe("Scanning");
    expect(next.label("zh-Hans")).toBe("扫描");
    expect(next.completed).toBe(2);
    expect(next.total).toBe(3);
    expect(progress.completed).toBe(1);
    expect(format).toHaveBeenCalledTimes(2);
});

test("progress rejects invalid publication while the retained label stays usable", () =>
{
    const progress = { label: { message: "scan" }, completed: 1, total: 3 };
    const resources = _resources("界".repeat(42));
    const prepared = new ProgressPresentation(progress, resources, "macos");
    expect(() => new ProgressPresentation(progress, _resources("界".repeat(43)), "macos")).toThrow("128 bytes");
    expect(() => new ProgressPresentation(progress, undefined, "macos")).toThrow();
    expect(() => new ProgressPresentation({ ...progress, label: { message: "missing" } }, resources, "macos", prepared))
        .toThrow();
    for (const invalid of [{ completed: 4, total: 3 }, { completed: 1, total: null }, { completed: -1, total: 3 }])
    {
        expect(() => new ProgressPresentation({ ...progress, ...invalid }, resources, "macos", prepared)).toThrow();
    }
    expect(prepared.label("en")).toBe("Scanning");
});

function _resources(chinese: string): ResourceSet
{
    return new ResourceSet({
        identity: "a".repeat(64),
        defaultLocale: "en",
        locales: {
            en: { sources: ["scan = Scanning"], contracts: { scan: { arguments: [] } } },
            "zh-Hans": { sources: [`scan = ${chinese}`], contracts: { scan: { arguments: [] } } }
        }
    });
}
