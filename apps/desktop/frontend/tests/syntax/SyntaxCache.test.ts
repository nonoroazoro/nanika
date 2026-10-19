import { expect, it } from "vitest";

import { SyntaxCache } from "../../src/syntax/SyntaxCache";

const identity = { scope: "session/instance", id: "a", revision: "1", theme: "light" as const };
const snapshot = {
    processed: 12,
    result: { language: "javascript" as const, ranges: new Uint32Array([0, 5, 2]) }
};

it("accounts tokens, touches LRU, and revokes revisions/scopes", () =>
{
    const cache = new SyntaxCache(32 * 1024, 2);
    cache.put(identity, snapshot);
    cache.put({ ...identity, id: "b" }, snapshot);
    expect(cache.get(identity)).toBe(snapshot);
    cache.put({ ...identity, id: "c" }, snapshot);
    expect(cache.get({ ...identity, id: "b" })).toBeNull();
    cache.invalidate(identity.scope, identity.id, "2");
    expect(cache.get(identity)).toBeNull();
    expect(cache.bytes).toBeGreaterThan(snapshot.result.ranges.byteLength);
    cache.invalidate(identity.scope);
    expect(cache.bytes).toBe(0);
    expect(cache.size).toBe(0);
});

it("does not admit a single entry beyond the entire byte budget", () =>
{
    const cache = new SyntaxCache(32);
    cache.put(identity, snapshot);
    expect(cache.get(identity)).toBeNull();
    expect(cache.bytes).toBe(0);
});

it("keeps theme variants separate while revoking both on a document revision change", () =>
{
    const cache = new SyntaxCache();
    const dark = { ...identity, theme: "dark" as const };
    const darkSnapshot = { ...snapshot, result: { ...snapshot.result, ranges: new Uint32Array([0, 5, 22]) } };
    cache.put(identity, snapshot);
    expect(cache.get(dark)).toBeNull();
    cache.put(dark, darkSnapshot);
    expect(cache.get(identity)).toBe(snapshot);
    expect(cache.get(dark)).toBe(darkSnapshot);
    cache.invalidate(identity.scope, identity.id, "2");
    expect(cache.get(identity)).toBeNull();
    expect(cache.get(dark)).toBeNull();
});

it("reserves viewport measurements inside the same byte budget without charging plain results", () =>
{
    const cache = new SyntaxCache(1024);
    cache.put(identity, snapshot);
    expect(cache.get(identity)).toBeNull();
    const plain = {
        processed: snapshot.processed,
        result: { language: "css" as const, ranges: new Uint32Array() }
    };
    cache.put(identity, plain);
    expect(cache.get(identity)).toBe(plain);
    expect(cache.bytes).toBeLessThanOrEqual(1024);
});
