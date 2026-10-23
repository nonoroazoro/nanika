import { expect, test, vi } from "vitest";

import { ResourceSet } from "../../src/localization/ResourceSet";
import { TextPresentation } from "../../src/localization/TextPresentation";
import { viewFields } from "../../src/localization/view-fields";
import { clipboardResources } from "../../verification/fixtures/localization";
import { localizedList } from "../../verification/fixtures/view";

import type { View } from "../../src/generated/View";

test("list presentation retains labels and domain identity without formatting on language changes", () =>
{
    const view = structuredClone(localizedList);
    const before = structuredClone(view);
    const resources = new ResourceSet(clipboardResources);
    const format = vi.spyOn(resources, "format");
    const prepared = new TextPresentation(resources, "macos", viewFields(view));
    const calls = format.mock.calls.length;
    expect(prepared.text({ message: "extension-name" }, "zh-Hans")).toBe("剪贴板历史");
    expect(prepared.text({ message: "view-search-placeholder" }, "zh-Hans")).toBe("搜索剪贴板历史");
    expect(prepared.text({ message: "content-text" }, "zh-Hans")).toBe("文本");
    expect(prepared.text({ message: "metadata-content-type" }, "zh-Hans")).toBe("内容类型");
    expect(prepared.text({ message: "filter-all" }, "en")).toBe("All Types");
    expect(format).toHaveBeenCalledTimes(calls);
    expect(view).toEqual(before);
});

test("offscreen selection and image alternatives are admitted before replacing retained presentation", () =>
{
    const resources = new ResourceSet(clipboardResources);
    const retained = new TextPresentation(resources, "macos", viewFields(localizedList));
    const view: View = structuredClone(localizedList);
    if (view.kind !== "list")
    {
        throw new Error("Expected list fixture");
    }
    const section = view.list.sections[0];
    if (!section)
    {
        throw new Error("Expected section fixture");
    }
    section.total = 100;
    view.list.selection = {
        index: 99,
        item: { id: "offscreen", title: { message: "unknown" }, subtitle: null, icon: null, actions: [] }
    };
    expect(() => new TextPresentation(resources, "macos", viewFields(view))).toThrow();
    view.list.selection.item.title = { message: "content-text" };
    view.list.detail = {
        title: null,
        content: { kind: "image", source: { path: `${"a".repeat(64)}.png` }, alternative_text: { message: "unknown" } },
        metadata: [],
        actions: []
    };
    expect(() => new TextPresentation(resources, "macos", viewFields(view))).toThrow();
    if (view.list.detail.content.kind !== "image")
    {
        throw new Error("Expected image fixture");
    }
    view.list.detail.content.alternative_text = { message: "content-image" };
    const accepted = new TextPresentation(resources, "macos", viewFields(view));
    expect(accepted.text({ message: "content-image" }, "zh-Hans")).toBe("图片");
    expect(retained.text({ message: "view-empty-title" }, "zh-Hans")).toBe("暂无剪贴板历史");
});
