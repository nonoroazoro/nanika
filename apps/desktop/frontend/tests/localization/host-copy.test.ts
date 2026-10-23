import { expect, test } from "vitest";

import { hostResources } from "../../src/generated/host-resources";
import { ResourceSet } from "../../src/localization/ResourceSet";

const resources = new ResourceSet(hostResources);

test("host counts select English plurals and retain numeric facts in Chinese", () =>
{
    for (
        const [message, singular, plural, chinese] of [
            ["settings-search-count", "1 result", "2 results", "2 个结果"],
            ["launcher-searching-count", "Searching 1 extension…", "Searching 2 extensions…", "正在搜索 2 个扩展…"],
            ["view-file-count", "1 item", "2 items", "2 个项目"],
            ["view-file-collection-label", "Collection of 1 item", "Collection of 2 items", "包含 2 个项目的集合"],
            ["view-files-more", "1 more file", "2 more files", "还有 2 个文件"],
            ["view-file-paths", "Path", "Paths", "路径"]
        ] as const
    )
    {
        for (
            const [locale, count, expected] of [["en", 1, singular], ["en", 2, plural], [
                "zh-Hans",
                2,
                chinese
            ]] as const
        )
        {
            const text = resources.format(locale, { message, args: { count } }, "macos", 2048);
            expect(text.replace(/[\u2068\u2069]/g, "")).toBe(expected);
        }
    }
});

test("shortcut instructions follow the platform independently of language", () =>
{
    for (const locale of ["en", "zh-Hans"] as const)
    {
        const macos = resources.format(locale, { message: "settings-shortcut-modifiers" }, "macos", 2048);
        const windows = resources.format(locale, { message: "settings-shortcut-modifiers" }, "windows", 2048);
        expect(macos).toContain("Command");
        expect(macos).not.toContain("Windows");
        expect(windows).toContain("Windows");
        expect(windows).not.toContain("Command");
    }
});
