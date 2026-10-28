import assert from "node:assert/strict";
import { expect, test, vi } from "vitest";

import { ResourceCache } from "../../src/localization/ResourceCache";
import { prepareRootPresentation } from "../../src/localization/root-presentation";

import type { LocalizationResourceRequest } from "../../src/generated/LocalizationResourceRequest";
import type { SearchResult } from "../../src/generated/SearchResult";

test("mounted owners survive cache eviction while changed instance or session authority requires a read", async () =>
{
    const read = vi.fn(async (request: Extract<LocalizationResourceRequest, { surface: "launcher"; }>) => ({
        identity: request.scope.identity,
        defaultLocale: "en" as const,
        locales: {
            en: { sources: ["label = Run"], contracts: { label: { arguments: [] } } },
            "zh-Hans": { sources: ["label = 运行"], contracts: { label: { arguments: [] } } }
        }
    }));
    const cache = new ResourceCache(read);
    const session = { sessionId: 1, platform: "macos" as const };
    const results = Array.from({ length: 12 }, (_, index) => _result(`example.owner${index}`));
    const first = await prepareRootPresentation(results, session, new Map(), cache);
    expect(read).toHaveBeenCalledTimes(12);
    const second = await prepareRootPresentation(
        results.map(result => ({ ...result, entryId: "another" })),
        session,
        first,
        cache
    );
    expect(read).toHaveBeenCalledTimes(12);
    for (const [id, value] of second)
    {
        expect(value.presentation.resources).toBe(first.get(id)?.presentation.resources);
        expect(value.presentation.text({ message: "label" }, "zh-Hans")).toBe("运行");
    }
    const nextInstance = _result("example.owner0", 2);
    assert.ok(nextInstance.actions[0]);
    const third = await prepareRootPresentation([nextInstance], session, second, cache);
    expect(read).toHaveBeenCalledTimes(13);
    await prepareRootPresentation([nextInstance], { ...session, sessionId: 2 }, third, cache);
    expect(read).toHaveBeenCalledTimes(14);
    const foreign = { ...nextInstance, resourceScope: { extensionId: "foreign.owner", identity: "a".repeat(64) } };
    await expect(prepareRootPresentation([foreign], session, third, cache)).rejects.toThrow("owner");
    await expect(
        prepareRootPresentation(
            [{ ...nextInstance, actions: [{ ...nextInstance.actions[0], title: { message: "missing" } }] }],
            session,
            third,
            cache
        )
    )
        .rejects.toThrow();
    expect(third.get(nextInstance.extensionId)?.presentation.text(nextInstance.actions[0].title, "en")).toBe("Run");
});

test("host-admitted locale variants preserve the full result batch and warm language switching", async () =>
{
    const read = vi.fn(async () =>
    {
        throw new Error("Domain names need no package resources");
    });
    const cache = new ResourceCache(read);
    const results = ["en-posix", "jaw"].map((tag, index): SearchResult => ({
        ..._result("com.nanika.application"),
        resourceScope: null,
        entryId: `app.${index}`,
        actions: [{
            id: "run",
            title: "Open",
            style: "primary",
            enabled: true,
            allow_default_execution: true,
            group: null
        }],
        subtitle: null,
        title: {
            fallback: "Source",
            translations: { en: "Music", "zh-Hans": "音乐", [tag]: "Other name" }
        }
    }));
    const before = structuredClone(results);
    const prepared = await prepareRootPresentation(
        results,
        { sessionId: 1, platform: "macos" },
        new Map(),
        cache
    );
    const presentation = prepared.get("com.nanika.application")?.presentation;
    for (let index = 0; index < 100; index++)
    {
        for (const result of results)
        {
            expect(presentation?.text(result.title, "en")).toBe("Music");
            expect(presentation?.text(result.title, "zh-Hans")).toBe("音乐");
        }
    }
    expect(read).not.toHaveBeenCalled();
    expect(results).toEqual(before);
});

function _result(id: string, instanceId = 1): SearchResult
{
    return {
        extensionId: id,
        instanceId,
        resourceScope: { extensionId: id, identity: "a".repeat(64) },
        entryId: "entry",
        actionId: "run",
        actions: [{
            id: "run",
            title: { message: "label" },
            style: "destructive",
            enabled: true,
            allow_default_execution: false,
            confirmation_title: { message: "label" },
            group: null
        }],
        title: { message: "label" },
        subtitle: { kind: "label", text: { message: "label" } },
        icon: null,
        kind: "Extension",
        entryType: "action"
    };
}
