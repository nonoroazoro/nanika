import { describe, expect, it, vi } from "vitest";

import { TextDocumentModel } from "../../src/text/TextDocumentModel";

import type { TextRead } from "../../src/generated/TextRead";

const descriptor = {
    id: "a",
    revision: "r1",
    utf16Length: 16_385,
    lineCount: 1,
    maxLineLength: 16_385,
    totalChunks: 2
};

describe("independent text reads", () =>
{
    it("deduplicates concurrent demand without navigation receipts", async () =>
    {
        const source = new TextDocumentModel(descriptor, "a".repeat(16_384));
        const read = vi.fn(async (request: TextRead) => ({ ...request, value: "b" }));
        await Promise.all([source.readNext(read), source.readNext(read)]);
        expect(read).toHaveBeenCalledTimes(1);
        expect(source.complete).toBe(true);
        expect(source.chunks.join("")).toBe(`${"a".repeat(16_384)}b`);
    });

    it("rejects stale revisions and incorrect lengths without appending", async () =>
    {
        const source = new TextDocumentModel(descriptor, "a".repeat(16_384));
        await expect(source.readNext(async request => ({ ...request, revision: "old", value: "b" }))).rejects.toThrow();
        await expect(source.readNext(async request => ({ ...request, value: "bc" }))).rejects.toThrow();
        expect(source.chunks).toHaveLength(1);
        await source.readNext(async request => ({ ...request, value: "b" }));
        expect(source.complete).toBe(true);
    });

    it("ignores disposed responses and refuses further reads", async () =>
    {
        const source = new TextDocumentModel(descriptor, "a".repeat(16_384));
        const read = vi.fn(async (request: TextRead) => ({ ...request, value: "b" }));
        const pending = source.readNext(read);
        source.dispose();
        await pending;
        expect(source.chunks).toHaveLength(0);
        await expect(source.readNext(read)).rejects.toThrow("no longer available");
        expect(read).toHaveBeenCalledTimes(1);
    });

    it("ignores cancelled replies while allowing a fresh read of the same chunk", async () =>
    {
        const source = new TextDocumentModel(descriptor, "a".repeat(16_384));
        const stale = Promise.withResolvers<{ documentId: string; index: number; revision: string; value: string; }>();
        const pending = source.readNext(async () => stale.promise);
        source.cancelReads();
        await source.readNext(async request => ({ ...request, value: "b" }));
        stale.resolve({ documentId: "a", revision: "r1", index: 1, value: "x" });
        await pending;
        expect(source.chunks[1]).toBe("b");
        expect(source.complete).toBe(true);
    });
});

it("slices only resident chunks and preserves UTF-16 offsets across transport boundaries", async () =>
{
    const prefix = `${"a".repeat(16382)}😀`;
    const model = new TextDocumentModel({ ...descriptor, utf16Length: prefix.length + 3 }, `${prefix}x`);
    expect(model.slice(16381, 16388)).toBe("a😀x");
    await model.readNext(async request => ({ ...request, value: "yz" }));
    expect(model.slice(16381, 16388)).toBe("a😀xyz");
    expect(model.slice(0, 32)).toBe("a".repeat(32));
});
