import assert from "node:assert/strict";
import { test } from "vitest";

import { groupSearchResults } from "../../src/settings/groupSearchResults";

import type { SettingsSearchEntry } from "../../src/types/SettingsSearchEntry";

const field: SettingsSearchEntry = {
    pageId: "applications",
    pageTitle: "Applications",
    target: { kind: "field", key: "folders" },
    title: "Folders"
};

test("groups use navigation order while fields preserve rank without duplicate page destinations", () =>
{
    const scripts = { ...field, pageId: "scripts", pageTitle: "Scripts" };
    const enabled: SettingsSearchEntry = { ...field, target: { kind: "enabled" }, title: "Enable extension" };
    const page: SettingsSearchEntry = { ...field, target: { kind: "page" }, title: "Applications" };
    const results = [field, scripts, page, enabled];
    assert.deepEqual(groupSearchResults(results, ["scripts", "applications"]), [
        { pageId: "scripts", title: "Scripts", entries: [scripts] },
        { pageId: "applications", title: "Applications", entries: [field, enabled] }
    ]);
    assert.deepEqual(results, [field, scripts, page, enabled]);
});

test("page-only matches remain navigable headings and empty results produce no groups", () =>
{
    assert.deepEqual(
        groupSearchResults([{ ...field, target: { kind: "page" }, title: "Applications" }], [
            "general",
            "applications"
        ]),
        [
            { pageId: "applications", title: "Applications", entries: [] }
        ]
    );
    assert.deepEqual(groupSearchResults([], ["general", "applications"]), []);
});

test("General stays first regardless of match scores and unmatched pages remain absent", () =>
{
    const general: SettingsSearchEntry = {
        pageId: "general",
        pageTitle: "General",
        target: { kind: "field", key: "theme" },
        title: "Theme"
    };
    const order = ["general", "applications", "scripts"];
    for (const results of [[field, general], [general, field]])
    {
        assert.deepEqual(groupSearchResults(results, order).map(group => group.pageId), ["general", "applications"]);
    }
});
