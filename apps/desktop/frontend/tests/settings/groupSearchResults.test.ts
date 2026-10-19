import assert from "node:assert/strict";
import { test } from "vitest";

import { groupSearchResults } from "../../src/settings/groupSearchResults";

import type { SettingsSearchEntry } from "../../src/generated/SettingsSearchEntry";

const field: SettingsSearchEntry = {
    pageId: "applications",
    pageTitle: "Applications",
    target: { kind: "field", key: "folders" },
    title: "Folders"
};

test("groups and fields preserve host order without duplicate page destinations", () =>
{
    const scripts = { ...field, pageId: "scripts", pageTitle: "Scripts" };
    const enabled: SettingsSearchEntry = { ...field, target: { kind: "enabled" }, title: "Enable extension" };
    const page: SettingsSearchEntry = { ...field, target: { kind: "page" }, title: "Applications" };
    const results = [field, scripts, page, enabled];
    assert.deepEqual(groupSearchResults(results), [
        { pageId: "applications", title: "Applications", entries: [field, enabled] },
        { pageId: "scripts", title: "Scripts", entries: [scripts] }
    ]);
    assert.deepEqual(results, [field, scripts, page, enabled]);
});

test("page-only matches remain navigable headings and empty results produce no groups", () =>
{
    assert.deepEqual(
        groupSearchResults([{ ...field, target: { kind: "page" }, title: "Applications" }]),
        [
            { pageId: "applications", title: "Applications", entries: [] }
        ]
    );
    assert.deepEqual(groupSearchResults([]), []);
});

test("page groups are never independently sorted and unmatched pages remain absent", () =>
{
    const general: SettingsSearchEntry = {
        pageId: "general",
        pageTitle: "General",
        target: { kind: "field", key: "theme" },
        title: "Theme"
    };
    for (const results of [[field, general], [general, field]])
    {
        assert.deepEqual(groupSearchResults(results).map(group => group.pageId), results.map(entry => entry.pageId));
    }
});
