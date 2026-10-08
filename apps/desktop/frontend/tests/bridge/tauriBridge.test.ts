import assert from "node:assert/strict";
import { beforeEach, expect, expectTypeOf, test, vi } from "vitest";

import { TestSearchChannel } from "./TestSearchChannel";
import { tauriBridge } from "../../src/bridge/tauriBridge";

import type { NanikaBridge } from "../../src/bridge/NanikaBridge";
import type { RootSearchSnapshot as SearchUpdate } from "../../src/generated/RootSearchSnapshot";
import type { RootSearchSnapshot } from "../../src/types/RootSearchSnapshot";
import type { ViewInteraction } from "../../src/types/ViewInteraction";

const harness = vi.hoisted(() => ({
    invoke: vi.fn().mockResolvedValue(undefined)
}));

vi.mock("@tauri-apps/api/core", async () =>
{
    const { TestSearchChannel: Channel } = await import("./TestSearchChannel");
    return { invoke: harness.invoke, Channel };
});

beforeEach(() =>
{
    TestSearchChannel.instances.length = 0;
    harness.invoke.mockClear();
});

test("the frontend cannot send host-owned invalidation events", () =>
{
    expectTypeOf<{ kind: "invalidated"; }>().not.toExtend<ViewInteraction>();
    expectTypeOf<{ kind: "resumed"; }>().toExtend<ViewInteraction>();
    expectTypeOf<{
        operation: { event: { kind: "invalidated"; }; kind: "event"; };
        revision: number;
        routeId: number;
        sessionId: number;
    }>().not.toExtend<Parameters<NanikaBridge["viewEvent"]>[0]>();
});

test("wire omissions retain results and navigation identity; null explicitly returns to root", async () =>
{
    const snapshots: RootSearchSnapshot[] = [];
    const onError = vi.fn<(error: unknown) => void>();
    await tauriBridge.openSession(snapshot =>
    {
        snapshots.push(snapshot);
    }, onError);
    const channel = TestSearchChannel.instances[0];
    assert.ok(channel);
    const initial = _update();
    initial.navigation.current = {
        routeId: 1,
        extensionId: "test.extension",
        instanceId: 1,
        generation: 1,
        viewId: "detail",
        revision: 1,
        view: {
            kind: "detail",
            detail: {
                title: null,
                actions: [],
                metadata: [],
                content: { kind: "text", text_id: "text", chunk_index: 0, total_chunks: 1, value: "Text" }
            }
        }
    };
    channel.onmessage(initial);
    const delta = _update();
    delta.revision = 2;
    delete delta.results;
    delete delta.navigation.current;
    channel.onmessage(delta);
    expect(snapshots[1]?.results).toBe(snapshots[0]?.results);
    expect(snapshots[1]?.navigation.current).toBe(snapshots[0]?.navigation.current);
    channel.onmessage({ ...delta, revision: 3, navigation: { ...delta.navigation, current: null } });
    expect(snapshots[2]?.navigation.current).toBeNull();
    expect(harness.invoke).toHaveBeenLastCalledWith("acknowledge_search", { sessionId: 1, revision: 3 });
    expect(onError).not.toHaveBeenCalled();
});

test("a partial first delivery is rejected without acknowledgement", async () =>
{
    const listener = vi.fn<(snapshot: RootSearchSnapshot) => void>();
    const onError = vi.fn<(error: unknown) => void>();
    await tauriBridge.openSession(listener, onError);
    const partial = _update();
    delete partial.results;
    const channel = TestSearchChannel.instances[0];
    assert.ok(channel);
    channel.onmessage(partial);
    expect(listener).not.toHaveBeenCalled();
    expect(onError).toHaveBeenCalledOnce();
    expect(harness.invoke).not.toHaveBeenCalledWith("acknowledge_search", expect.anything());
});

test("the bridge supplies range IDs and resets their sequence when opening a session", async () =>
{
    const request = { sessionId: 1, requestId: 1, resultRevision: 1, offset: 0, count: 10 };
    await tauriBridge.openSession(vi.fn<(snapshot: RootSearchSnapshot) => void>(), vi.fn<(error: unknown) => void>());
    await tauriBridge.readResults(request);
    expect(harness.invoke).toHaveBeenLastCalledWith("read_results", { request: { ...request, rangeId: 1 } });
    await tauriBridge.readResults(request);
    expect(harness.invoke).toHaveBeenLastCalledWith("read_results", { request: { ...request, rangeId: 2 } });
    await tauriBridge.openSession(vi.fn<(snapshot: RootSearchSnapshot) => void>(), vi.fn<(error: unknown) => void>());
    await tauriBridge.readResults(request);
    expect(harness.invoke).toHaveBeenLastCalledWith("read_results", { request: { ...request, rangeId: 1 } });
});

function _update(): SearchUpdate
{
    return {
        navigation: { revision: 1, current: null, busy: false, error: null, dismissCount: 0 },
        sessionId: 1,
        requestId: 1,
        revision: 1,
        query: "",
        results: [],
        resultRevision: 1,
        resultOffset: 0,
        totalResults: 0,
        phase: "ready",
        error: null,
        warnings: [],
        pendingExtensions: []
    };
}
