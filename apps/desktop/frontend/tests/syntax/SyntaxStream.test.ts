import { expect, it, vi } from "vitest";

import { SyntaxCache } from "../../src/syntax/SyntaxCache";
import { SyntaxStream } from "../../src/syntax/SyntaxStream";
import { TextDocumentModel } from "../../src/text/TextDocumentModel";

import type { SyntaxContinuation } from "../../src/syntax/SyntaxContinuation";
import type { SyntaxProgress } from "../../src/syntax/SyntaxProgress";
import type { SyntaxResult } from "../../src/syntax/SyntaxResult";
import type { SyntaxService } from "../../src/syntax/SyntaxService";

const identity = { scope: "view", id: "document", revision: "1", theme: "light" as const };

function harness(cached = 0, length = 32768)
{
    const closeStream = vi.fn();
    const detect = vi.fn(async () => "css");
    const append = vi.fn(async (
        _identity: unknown,
        text: string,
        _language: unknown,
        _signal: AbortSignal,
        cursor: SyntaxContinuation
    ): Promise<SyntaxResult> => ({
        language: "css",
        ranges: new Uint32Array([cursor.offset, cursor.offset + 1, 2])
    }));
    const cache = new SyntaxCache();
    if (cached)
    {
        cache.put(identity, {
            processed: cached,
            result: { language: "css", ranges: new Uint32Array([0, 1, 2]) }
        });
    }
    const service = {
        lookup: (id: typeof identity) => cache.get(id),
        openStream: () => 1,
        closeStream,
        detect,
        append,
        cache
    } as unknown as SyntaxService;
    const model = new TextDocumentModel({
        id: "document",
        revision: "1",
        utf16Length: length,
        lineCount: 1,
        maxLineLength: length,
        totalChunks: Math.ceil(length / 16384)
    }, "a".repeat(Math.min(length, 16384)));
    const publish = vi.fn((_snapshot: SyntaxProgress): void =>
    {});
    const stream = new SyntaxStream(service, identity, model, publish);
    return { stream, append, detect, closeStream, model, publish };
}

it("processes only a small visible prefix, remains idle, and continues on scroll without reading source", async () =>
{
    const { stream, append, detect, closeStream, model } = harness();
    stream.advance(300);
    await vi.waitFor(() =>
    {
        expect(append).toHaveBeenCalledTimes(1);
    });
    expect(append.mock.calls[0]?.[1]).toHaveLength(2048);
    await new Promise<void>(resolve =>
    {
        setTimeout(resolve, 10);
    });
    expect(append).toHaveBeenCalledTimes(1);
    stream.advance(2500);
    await vi.waitFor(() =>
    {
        expect(append).toHaveBeenCalledTimes(2);
    });
    expect(append.mock.calls[1]?.[4].offset).toBe(2048);
    expect(detect).toHaveBeenCalledTimes(1);
    expect(model.chunks).toHaveLength(1);
    stream.dispose();
    stream.advance(8000);
    expect(append).toHaveBeenCalledTimes(2);
    expect(closeStream).toHaveBeenCalledOnce();
});

it("suppresses an in-flight result after disposal", async () =>
{
    const { stream, append, publish } = harness();
    let finish: ((result: SyntaxResult) => void) | undefined;
    append.mockImplementation(async () =>
    {
        return new Promise(resolve =>
        {
            finish = resolve;
        });
    });
    stream.advance(300);
    await vi.waitFor(() =>
    {
        expect(append).toHaveBeenCalledOnce();
    });
    stream.dispose();
    expect(append.mock.calls[0]?.[3].aborted).toBe(true);
    finish?.({ language: "css", ranges: new Uint32Array([0, 1, 2]) });
    await Promise.resolve();
    await Promise.resolve();
    expect(publish).not.toHaveBeenCalled();
});

it("keeps a cached viewport idle and reconstructs grammar state only when demand passes it", async () =>
{
    const { stream, append, detect } = harness(4096);
    stream.advance(300);
    await Promise.resolve();
    expect(append).not.toHaveBeenCalled();
    expect(detect).not.toHaveBeenCalled();
    stream.advance(5000);
    await vi.waitFor(() =>
    {
        expect(append).toHaveBeenCalledTimes(3);
    });
    expect(append.mock.calls.map(call => call[4].offset)).toEqual([0, 2048, 4096]);
    expect(detect).not.toHaveBeenCalled();
    stream.dispose();
});

it("finishes an already loaded final line across batches without waiting for a scroll", async () =>
{
    const { stream, append, publish } = harness(0, 3001);
    stream.advance(300);
    await vi.waitFor(() =>
    {
        expect(publish).toHaveBeenCalledTimes(2);
    });
    expect(append.mock.calls.map(call => call[1].length)).toEqual([2048, 953]);
    expect(append.mock.calls[1]?.[4]).toEqual({ stream: 1, offset: 2048, final: true });
    stream.dispose();
});
