import { afterEach, expect, it, vi } from "vitest";

import { TOKEN_COLORS } from "../../src/syntax/palette";
import { SyntaxService } from "../../src/syntax/SyntaxService";

import type { SyntaxWorkerRequest } from "../../src/syntax/SyntaxWorkerRequest";
import type { SyntaxWorkerResponse } from "../../src/syntax/SyntaxWorkerResponse";

function workerHarness()
{
    const workers: Array<{ post: ReturnType<typeof vi.fn>; terminate: ReturnType<typeof vi.fn>; worker: Worker; }> = [];
    const service = new SyntaxService(() =>
    {
        const post = vi.fn();
        const terminate = vi.fn();
        const worker = {
            postMessage: post,
            terminate,
            onmessage: null,
            onerror: null,
            onmessageerror: null
        } as unknown as Worker;
        workers.push({ worker, post, terminate });
        return worker;
    });
    const emit = (message: SyntaxWorkerResponse, index = workers.length - 1): void =>
    {
        const worker = workers[index]?.worker;
        worker?.onmessage?.call(worker, { data: message } as MessageEvent<SyntaxWorkerResponse>);
    };
    return { service, workers, emit };
}

const identity = { scope: "s1/e1", id: "doc", revision: "1", theme: "light" as const };
const result = { language: "javascript" as const, ranges: new Uint32Array([0, 5, 2]) };

afterEach(() => vi.useRealTimers());

it("deduplicates stream batches and aborts one subscriber independently", async () =>
{
    const { service, workers, emit } = workerHarness();
    const first = new AbortController();
    const second = new AbortController();
    const a = _append(service, identity, "const x = 1;", first.signal);
    const b = _append(service, identity, "const x = 1;", second.signal);
    expect(workers).toHaveLength(1);
    emit({ kind: "ready" });
    expect(workers[0]?.post).toHaveBeenCalledTimes(1);
    first.abort();
    expect(await a).toBeNull();
    expect(workers[0]?.post).toHaveBeenCalledTimes(1);
    emit({ kind: "result", id: 1, result });
    expect(await b).toBe(result);
    expect(service.lookup(identity)).toBeNull();
    service.suspend();
});

it("cancels obsolete work cooperatively, reuses the worker, and suppresses stale publication", async () =>
{
    const { service, workers, emit } = workerHarness();
    const abort = new AbortController();
    const first = _append(service, identity, "const x = 1;", abort.signal);
    emit({ kind: "ready" });
    abort.abort();
    const next = _append(service, { ...identity, revision: "2" }, "const x = 2;", new AbortController().signal);
    emit({ kind: "result", id: 1, result });
    expect(await first).toBeNull();
    expect(service.lookup(identity)).toBeNull();
    expect(workers).toHaveLength(1);
    expect(workers[0]?.post).toHaveBeenLastCalledWith(expect.objectContaining({ kind: "tokenize", id: 2 }));
    emit({ kind: "result", id: 2, result });
    expect(await next).not.toBeNull();
    service.suspend();
});

it("terminates unresponsive cancellation and leaves no hidden timers or work", async () =>
{
    vi.useFakeTimers();
    const { service, workers, emit } = workerHarness();
    const abort = new AbortController();
    const request = _append(service, identity, "const x = 1;", abort.signal);
    emit({ kind: "ready" });
    abort.abort();
    await vi.advanceTimersByTimeAsync(150);
    expect(await request).toBeNull();
    expect(workers[0]?.terminate).toHaveBeenCalledOnce();
    const second = _append(service, identity, "const x = 1;", new AbortController().signal);
    service.suspend();
    expect(await second).toBeNull();
    expect(workers[1]?.terminate).toHaveBeenCalledOnce();
    expect(vi.getTimerCount()).toBe(0);
});

it("invalidation revokes pending and resident data without accepting a late result", async () =>
{
    const { service, emit } = workerHarness();
    const request = _append(service, identity, "const x = 1;", new AbortController().signal);
    emit({ kind: "ready" });
    service.invalidate(identity.scope, identity.id);
    expect(await request).toBeNull();
    emit({ kind: "result", id: 1, result });
    expect(service.lookup(identity)).toBeNull();
    service.suspend();
});

it("bounds initialization failures and never schedules automatic retries", async () =>
{
    vi.useFakeTimers();
    const report = vi.spyOn(console, "error").mockImplementation(() => undefined);
    try
    {
        const { service, workers } = workerHarness();
        for (let attempt = 0; attempt < 2; attempt++)
        {
            const pending = _append(service, identity, "const x = 1;", new AbortController().signal);
            await vi.advanceTimersByTimeAsync(5_000);
            expect(await pending).toBeNull();
            expect(workers[attempt]?.terminate).toHaveBeenCalledOnce();
        }
        expect(await _append(service, identity, "const x = 1;", new AbortController().signal)).toBeNull();
        expect(workers).toHaveLength(2);
        expect(report).toHaveBeenCalledOnce();
        expect(vi.getTimerCount()).toBe(0);
    }
    finally
    {
        report.mockRestore();
    }
});

it("rejects invalid token offsets before caching or publishing", async () =>
{
    const { service, emit } = workerHarness();
    const request = _append(service, identity, "const x = 1;", new AbortController().signal);
    emit({ kind: "ready" });
    emit({ kind: "result", id: 1, result: { ...result, ranges: new Uint32Array([0, 99, 2]) } });
    expect(await request).toBeNull();
    expect(service.lookup(identity)).toBeNull();
    service.suspend();
});

it.each([
    null,
    { ...result, ranges: [0, 5, 2] },
    { ...result, ranges: new Uint32Array([0, 5, 0]) },
    { ...result, ranges: new Uint32Array([0, 5, TOKEN_COLORS.length]) },
    { ...result, language: null },
    { ...result, language: "unsupported" }
])("rejects malformed stream results: %j", async invalid =>
{
    const { service, emit } = workerHarness();
    const pending = _append(service, identity, "const x = 1;", new AbortController().signal);
    emit({ kind: "ready" });
    emit({ kind: "result", id: 1, result: invalid } as SyntaxWorkerResponse);
    expect(await pending).toBeNull();
    expect(service.lookup(identity)).toBeNull();
    service.suspend();
});

it("does not let a duplicate ready message disable an active job watchdog", async () =>
{
    vi.useFakeTimers();
    const report = vi.spyOn(console, "error").mockImplementation(() => undefined);
    try
    {
        const { service, emit, workers } = workerHarness();
        const pending = _append(service, identity, "const x = 1;", new AbortController().signal);
        emit({ kind: "ready" });
        emit({ kind: "tokenizing", id: 1 });
        emit({ kind: "ready" });
        await vi.advanceTimersByTimeAsync(1_500);
        expect(await pending).toBeNull();
        expect(workers[0]?.terminate).toHaveBeenCalledOnce();
        expect(vi.getTimerCount()).toBe(0);
    }
    finally
    {
        report.mockRestore();
    }
});

it("bounds queued source payloads and releases work with the final consumer", async () =>
{
    const { service, workers } = workerHarness();
    const release = service.retain();
    const requests = Array.from(
        { length: 10 },
        async (_, index) =>
            _append(service, { ...identity, id: String(index) }, "const x = 1;", new AbortController().signal)
    );
    expect(await requests[0]).toBeNull();
    expect(await requests[1]).toBeNull();
    release();
    release();
    expect(await Promise.all(requests)).toEqual(Array.from({ length: 10 }, () => null));
    expect(workers[0]?.terminate).toHaveBeenCalledOnce();
});

it("ignores late errors from a released Worker without interrupting its replacement", async () =>
{
    const { service, workers, emit } = workerHarness();
    const first = _append(service, identity, "const x = 1;", new AbortController().signal);
    const old = workers[0]?.worker;
    if (!old)
    {
        throw new Error("Expected a Worker for the first request.");
    }
    const lateError = old.onerror;
    const lateMessageError = old.onmessageerror;
    service.suspend();
    expect(await first).toBeNull();
    expect(old.onmessage).toBeNull();
    expect(old.onerror).toBeNull();
    expect(old.onmessageerror).toBeNull();

    const second = _append(service, identity, "const x = 1;", new AbortController().signal);
    emit({ kind: "ready" });
    lateError?.call(old, new Event("error") as ErrorEvent);
    lateMessageError?.call(old, new MessageEvent("messageerror"));
    expect(workers[1]?.terminate).not.toHaveBeenCalled();
    emit({ kind: "result", id: 2, result });
    expect(await second).toBe(result);
    service.suspend();
});

it("keeps shared demand alive until the last consumer releases", async () =>
{
    vi.useFakeTimers();
    const { service, workers, emit } = workerHarness();
    const releaseFirst = service.retain();
    const releaseSecond = service.retain();
    const pending = _append(service, identity, "const x = 1;", new AbortController().signal);
    emit({ kind: "ready" });
    releaseFirst();
    expect(workers[0]?.terminate).not.toHaveBeenCalled();
    emit({ kind: "result", id: 1, result });
    expect(await pending).toBe(result);
    releaseSecond();
    expect(workers[0]?.terminate).toHaveBeenCalledOnce();
    expect(service.lookup(identity)).toBeNull();
    expect(vi.getTimerCount()).toBe(0);
});

it.each(["tokenize", "cancel", "invalidate"])("releases demand and timers when %s delivery throws", async operation =>
{
    vi.useFakeTimers();
    const report = vi.spyOn(console, "error").mockImplementation(() => undefined);
    const { service, workers, emit } = workerHarness();
    try
    {
        const controller = new AbortController();
        const pending = _append(service, identity, "const x = 1;", controller.signal);
        if (operation !== "tokenize")
        {
            emit({ kind: "ready" });
        }
        expect(workers).toHaveLength(1);
        workers[0]?.post.mockImplementation(() =>
        {
            throw new Error("Worker delivery failed.");
        });
        if (operation === "tokenize")
        {
            emit({ kind: "ready" });
        }
        else if (operation === "cancel")
        {
            controller.abort();
        }
        else
        {
            service.invalidate(identity.scope, identity.id);
        }
        expect(await pending).toBeNull();
        expect(service.lookup(identity)).toBeNull();
        expect(workers[0]?.terminate).toHaveBeenCalledOnce();
        expect(report).toHaveBeenCalledOnce();
        expect(vi.getTimerCount()).toBe(0);

        // One later explicit request can recover; the failed request is never retried.
        const recovery = _append(service, identity, "const x = 1;", new AbortController().signal);
        emit({ kind: "ready" });
        emit({ kind: "result", id: 2, result });
        expect(await recovery).toBe(result);
        expect(vi.getTimerCount()).toBe(0);
    }
    finally
    {
        service.suspend();
        report.mockRestore();
    }
});

it("counts consecutive failures rather than failures separated by successful work", async () =>
{
    vi.useFakeTimers();
    const report = vi.spyOn(console, "error").mockImplementation(() => undefined);
    const { service, workers, emit } = workerHarness();
    const release = service.retain();
    try
    {
        for (let cycle = 0; cycle < 3; cycle++)
        {
            const document = { ...identity, id: `document-${cycle}` };
            const failed = _append(service, document, "const x = 1;", new AbortController().signal);
            emit({ kind: "error", id: null });
            expect(await failed).toBeNull();
            const recovered = _append(service, document, "const x = 1;", new AbortController().signal);
            emit({ kind: "ready" });
            emit({ kind: "result", id: (cycle * 2) + 2, result });
            expect(await recovered).toBe(result);
        }
        expect(workers).toHaveLength(4);
        expect(vi.getTimerCount()).toBe(0);
    }
    finally
    {
        release();
        report.mockRestore();
    }
});

it("allows a fresh consumer session to recover without retrying work or resetting an active session", async () =>
{
    vi.useFakeTimers();
    const report = vi.spyOn(console, "error").mockImplementation(() => undefined);
    const { service, workers, emit } = workerHarness();
    const release = service.retain();
    let releaseReopened = (): void =>
    {};
    try
    {
        for (let attempt = 0; attempt < 2; attempt++)
        {
            const pending = _append(service, identity, "const x = 1;", new AbortController().signal);
            emit({ kind: "error", id: null });
            expect(await pending).toBeNull();
        }
        const releaseAdditional = service.retain();
        expect(await _append(service, identity, "const x = 1;", new AbortController().signal)).toBeNull();
        expect(workers).toHaveLength(2);
        releaseAdditional();
        release();
        releaseReopened = service.retain();
        expect(workers).toHaveLength(2);
        expect(vi.getTimerCount()).toBe(0);
        const recovered = _append(service, identity, "const x = 1;", new AbortController().signal);
        emit({ kind: "ready" });
        emit({ kind: "result", id: 4, result });
        expect(await recovered).toBe(result);
        expect(workers).toHaveLength(3);
    }
    finally
    {
        release();
        releaseReopened();
        report.mockRestore();
    }
});

it("sends only the detection prefix regardless of document size", async () =>
{
    vi.useFakeTimers();
    const { service, workers, emit } = workerHarness();
    const source = `const value = "${"x".repeat(100_000)}`;
    const pending = service.detect(identity, source, new AbortController().signal);
    emit({ kind: "ready" });
    expect(workers[0]?.post).toHaveBeenCalledExactlyOnceWith({ kind: "detect", id: 1, text: source.slice(0, 4096) });
    expect(vi.getTimerCount()).toBe(0);
    emit({ kind: "detected", id: 1, language: "javascript" });
    expect(await pending).toBe("javascript");
    // A detection result is not a complete source/token snapshot.
    expect(service.lookup(identity)).toBeNull();
    service.suspend();
});

it("cancels detection on navigation and ignores the obsolete language without replacing the worker", async () =>
{
    const { service, workers, emit } = workerHarness();
    const abort = new AbortController();
    const first = service.detect(identity, "const a = 1;", abort.signal);
    emit({ kind: "ready" });
    abort.abort();
    const second = service.detect({ ...identity, id: "next" }, "fn main() {}", new AbortController().signal);
    expect(await first).toBeNull();
    emit({ kind: "detected", id: 1, language: "javascript" });
    expect(workers).toHaveLength(1);
    expect(workers[0]?.post).toHaveBeenLastCalledWith({ kind: "detect", id: 2, text: "fn main() {}" });
    emit({ kind: "detected", id: 2, language: "rust" });
    expect(await second).toBe("rust");
    service.suspend();
});

it("keeps detection and tokenization distinct in the shared queue", async () =>
{
    const { service, workers, emit } = workerHarness();
    const signal = new AbortController().signal;
    const detection = service.detect(identity, "const a = 1;", signal);
    const tokens = _append(service, identity, "const a = 1;", signal);
    emit({ kind: "ready" });
    emit({ kind: "detected", id: 1, language: "javascript" });
    expect(await detection).toBe("javascript");
    expect(workers[0]?.post).toHaveBeenLastCalledWith(expect.objectContaining({ kind: "tokenize", id: 2 }));
    emit({ kind: "result", id: 2, result });
    expect(await tokens).toBe(result);
    expect(workers).toHaveLength(1);
    service.suspend();
});

it("revokes queued detection before it reaches the worker", async () =>
{
    const { service, workers, emit } = workerHarness();
    const active = _append(service, identity, "const a = 1;", new AbortController().signal);
    emit({ kind: "ready" });
    const abort = new AbortController();
    const queued = service.detect({ ...identity, id: "next" }, "fn main() {}", abort.signal);
    abort.abort();
    expect(await queued).toBeNull();
    emit({ kind: "result", id: 1, result });
    await active;
    expect(workers[0]?.post).toHaveBeenCalledTimes(1);
    service.suspend();
});

it("revokes detection authority and rejects malformed language replies", async () =>
{
    const { service, emit } = workerHarness();
    const first = service.detect(identity, "const a = 1;", new AbortController().signal);
    emit({ kind: "ready" });
    service.invalidate(identity.scope);
    emit({ kind: "detected", id: 1, language: "javascript" });
    expect(await first).toBeNull();
    const second = service.detect(identity, "const a = 1;", new AbortController().signal);
    emit({ kind: "detected", id: 2, language: "__proto__" } as unknown as SyntaxWorkerResponse);
    expect(await second).toBeNull();
    service.suspend();
});

it("separates lazy engine initialization from tokenization and ignores duplicate start notifications", async () =>
{
    vi.useFakeTimers();
    const report = vi.spyOn(console, "error").mockImplementation(() => undefined);
    const { service, emit, workers } = workerHarness();
    try
    {
        const pending = _append(service, identity, "const a = 1;", new AbortController().signal);
        emit({ kind: "ready" });
        await vi.advanceTimersByTimeAsync(2_000);
        expect(workers[0]?.terminate).not.toHaveBeenCalled();
        emit({ kind: "tokenizing", id: 1 });
        await vi.advanceTimersByTimeAsync(1_000);
        emit({ kind: "tokenizing", id: 1 });
        await vi.advanceTimersByTimeAsync(500);
        expect(await pending).toBeNull();
        expect(workers[0]?.terminate).toHaveBeenCalledOnce();
        expect(vi.getTimerCount()).toBe(0);
    }
    finally
    {
        service.suspend();
        report.mockRestore();
    }
});

it("does not let a late tokenization start extend cancellation", async () =>
{
    vi.useFakeTimers();
    const { service, emit, workers } = workerHarness();
    const abort = new AbortController();
    const pending = _append(service, identity, "const a = 1;", abort.signal);
    emit({ kind: "ready" });
    abort.abort();
    emit({ kind: "tokenizing", id: 1 });
    await vi.advanceTimersByTimeAsync(150);
    expect(await pending).toBeNull();
    expect(workers[0]?.terminate).toHaveBeenCalledOnce();
    expect(vi.getTimerCount()).toBe(0);
});

it("releases detection listeners, deadlines and worker callbacks through repeated entry changes", async () =>
{
    vi.useFakeTimers();
    const { service, emit, workers } = workerHarness();
    const release = service.retain();
    for (let index = 0; index < 1_000; index++)
    {
        const controller = new AbortController();
        const remove = vi.spyOn(controller.signal, "removeEventListener");
        const pending = service.detect({ ...identity, id: `entry-${index}` }, "fn main() {}", controller.signal);
        if (index === 0)
        {
            emit({ kind: "ready" });
        }
        if (index % 2 === 0)
        {
            controller.abort();
        }
        emit({ kind: "detected", id: index + 1, language: "rust" });
        expect(await pending).toBe(index % 2 === 0 ? null : "rust");
        expect(remove).toHaveBeenCalledOnce();
        remove.mockRestore();
        expect(vi.getTimerCount()).toBe(0);
    }
    expect(workers).toHaveLength(1);
    expect(service.cache.size).toBe(0);
    release();
    expect(workers[0]?.terminate).toHaveBeenCalledOnce();
    expect(workers[0]?.worker.onmessage).toBeNull();
    expect(workers[0]?.worker.onerror).toBeNull();
    expect(workers[0]?.worker.onmessageerror).toBeNull();
    expect(vi.getTimerCount()).toBe(0);
});

it("keeps stream batches distinct and does not cache them as complete documents", async () =>
{
    const { service, workers, emit } = workerHarness();
    const stream = service.openStream();
    const signal = new AbortController().signal;
    const pending = service.append(identity, "const x = 1;", "javascript", signal, {
        stream,
        offset: 2048,
        final: false
    });
    emit({ kind: "ready" });
    const sent = workers[0]?.post.mock.calls[0]?.[0] as SyntaxWorkerRequest | undefined;
    if (sent?.kind !== "tokenize")
    {
        throw new Error("Expected a tokenization request.");
    }
    expect(sent.continuation).toEqual({ stream, offset: 2048, final: false });
    emit({ kind: "result", id: sent.id, result: { ...result, ranges: new Uint32Array([2048, 2053, 2]) } });
    expect((await pending)?.ranges).toEqual(new Uint32Array([2048, 2053, 2]));
    expect(service.lookup(identity)).toBeNull();
    service.closeStream(stream);
    expect(workers[0]?.post).toHaveBeenLastCalledWith({ kind: "release", stream });
    service.suspend();
});

async function _append(service: SyntaxService, document: typeof identity, text: string, signal: AbortSignal)
{
    return service.append(document, text, "javascript", signal, { stream: 1, offset: 0, final: true });
}
