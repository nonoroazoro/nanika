import assert from "node:assert/strict";
import { expect, test, vi } from "vitest";

import { MAX_CACHED_RESOURCE_SETS, MAX_RESOURCE_LOADS } from "../../src/generated/localization-limits";
import { ResourceCache } from "../../src/localization/ResourceCache";

import type { LocalizationResourceRequest } from "../../src/generated/LocalizationResourceRequest";
import type { PreparedResources } from "../../src/generated/PreparedResources";

test("concurrent readers share preparation and warm switching performs no resource reads", async () =>
{
    const reply = Promise.withResolvers<PreparedResources>();
    const read = vi.fn(async () => reply.promise);
    const cache = new ResourceCache(read);
    const request = _request(1);
    const first = cache.load(request);
    const second = cache.load(request);
    await Promise.resolve();
    expect(read).toHaveBeenCalledTimes(1);
    reply.resolve(_resources(request));
    const resources = await first;
    expect(await second).toBe(resources);
    for (let index = 0; index < 100; index++)
    {
        expect(resources.format(index % 2 ? "en" : "zh-Hans", { message: "title" }, "macos", 128))
            .toBe(request.scope.extensionId);
    }
    expect(await cache.load(request)).toBe(resources);
    expect(read).toHaveBeenCalledTimes(1);
});

test("cache eviction is bounded and does not change a retained surface's resources", async () =>
{
    const read = vi.fn(async (request: LocalizationResourceRequest) => _resources(request));
    const cache = new ResourceCache(read);
    const first = await cache.load(_request(0));
    for (let index = 1; index <= MAX_CACHED_RESOURCE_SETS; index++)
    {
        await cache.load(_request(index));
    }
    expect(first.format("en", { message: "title" }, "windows", 128)).toBe("com.example.package0");
    expect(await cache.load(_request(0))).not.toBe(first);
    expect(read).toHaveBeenCalledTimes(MAX_CACHED_RESOURCE_SETS + 2);
    const recent = await cache.load(_request(MAX_CACHED_RESOURCE_SETS));
    await cache.load(_request(MAX_CACHED_RESOURCE_SETS + 1));
    expect(await cache.load(_request(MAX_CACHED_RESOURCE_SETS))).toBe(recent);
    expect(read).toHaveBeenCalledTimes(MAX_CACHED_RESOURCE_SETS + 3);
});

test("owner, content, session, surface, and instance changes cannot reuse an old authorization", async () =>
{
    const read = vi.fn(async (request: LocalizationResourceRequest) => _resources(request));
    const cache = new ResourceCache(read);
    const initial = _request(1);
    const requests: LocalizationResourceRequest[] = [
        initial,
        { ...initial, scope: { ...initial.scope, extensionId: "com.example.another" } },
        { ...initial, scope: { ...initial.scope, identity: "f".repeat(64) } },
        { ...initial, sessionId: 2 },
        { surface: "launcher", sessionId: 1, instanceId: 1, scope: initial.scope },
        { surface: "launcher", sessionId: 1, instanceId: 2, scope: initial.scope }
    ];
    for (const request of requests)
    {
        await cache.load(request);
    }
    expect(read).toHaveBeenCalledTimes(requests.length);
    await cache.load(initial);
    expect(read).toHaveBeenCalledTimes(requests.length);
});

test("wrong or incomplete responses are retryable and leave prior presentation handles intact", async () =>
{
    const read = vi.fn(async (request: LocalizationResourceRequest) => _resources(request));
    const cache = new ResourceCache(read);
    const previous = await cache.load(_request(0));
    const request = _request(1);
    read.mockResolvedValueOnce(_resources(_request(2)));
    await expect(cache.load(request)).rejects.toMatchObject({ kind: "host", code: "localizationPackageChanged" });
    read.mockResolvedValueOnce({ ..._resources(request), locales: {} });
    await expect(cache.load(request)).rejects.toThrow("Default locale");
    read.mockRejectedValueOnce(new Error("transport failed"));
    await expect(cache.load(request)).rejects.toThrow("transport failed");
    const current = await cache.load(request);
    expect(current.format("en", { message: "title" }, "windows", 128)).toBe("com.example.package1");
    expect(previous.format("en", { message: "title" }, "windows", 128)).toBe("com.example.package0");
    expect(await cache.load(_request(0))).toBe(previous);
});

test("repeated instance changes reauthorize without accumulating copies of the same bundle", async () =>
{
    const read = vi.fn(async (request: LocalizationResourceRequest) => _resources(request));
    const cache = new ResourceCache(read);
    const scope = _request(1).scope;
    const retained = await cache.load({ surface: "launcher", sessionId: 1, instanceId: 1, scope });
    for (let instanceId = 2; instanceId <= 100; instanceId++)
    {
        expect(await cache.load({ surface: "launcher", sessionId: 1, instanceId, scope })).toBe(retained);
    }
    expect(read).toHaveBeenCalledTimes(100);
    // Old authority was evicted even though the immutable content is still retained.
    read.mockRejectedValueOnce(new Error("retired instance"));
    await expect(cache.load({ surface: "launcher", sessionId: 1, instanceId: 1, scope }))
        .rejects.toThrow("retired instance");
    expect(retained.format("en", { message: "title" }, "macos", 128)).toBe(scope.extensionId);
});

test("pending work is bounded, duplicate readers share slots, and failures release capacity", async () =>
{
    const replies = Array.from({ length: MAX_RESOURCE_LOADS }, () => Promise.withResolvers<PreparedResources>());
    const read = vi.fn(async (request: LocalizationResourceRequest) =>
    {
        const index = Number(request.scope.extensionId.replace("com.example.package", ""));
        return replies[index]?.promise ?? Promise.resolve(_resources(request));
    });
    const cache = new ResourceCache(read);
    const pending = replies.map(async (_, index) => cache.load(_request(index)));
    const duplicate = cache.load(_request(1));
    await expect(cache.load(_request(MAX_RESOURCE_LOADS))).rejects.toMatchObject({
        kind: "host",
        code: "localizationBusy"
    });
    expect(read).toHaveBeenCalledTimes(MAX_RESOURCE_LOADS);
    const rejected = expect(pending[0]).rejects.toThrow("transport failed");
    const firstReply = replies[0];
    assert.ok(firstReply);
    firstReply.reject(new Error("transport failed"));
    await rejected;
    await cache.load(_request(MAX_RESOURCE_LOADS));
    for (const [index, reply] of replies.slice(1).entries())
    {
        reply.resolve(_resources(_request(index + 1)));
    }
    expect(await duplicate).toBe(await pending[1]);
    await Promise.all(pending.slice(1));
    expect(read).toHaveBeenCalledTimes(MAX_RESOURCE_LOADS + 1);
});

test("a caller cannot mutate the scope while a resource read is in flight", async () =>
{
    const read = vi.fn(async (request: LocalizationResourceRequest) => _resources(request));
    const cache = new ResourceCache(read);
    const request = _request(1);
    const loading = cache.load(request);
    request.scope.identity = "f".repeat(64);
    request.scope.extensionId = "com.example.changed";
    const resources = await loading;
    expect(resources.identity).toBe(_request(1).scope.identity);
    expect(read).toHaveBeenCalledWith(_request(1));
});

function _request(index: number): Extract<LocalizationResourceRequest, { surface: "settings"; }>
{
    return {
        surface: "settings",
        sessionId: 1,
        scope: { extensionId: `com.example.package${index}`, identity: index.toString(16).padStart(64, "0") }
    };
}

function _resources(request: LocalizationResourceRequest): PreparedResources
{
    return {
        identity: request.scope.identity,
        defaultLocale: "en",
        locales: {
            en: { sources: [`title = ${request.scope.extensionId}`], contracts: { title: { arguments: [] } } }
        }
    };
}
