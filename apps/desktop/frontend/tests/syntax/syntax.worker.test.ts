import { afterEach, expect, it, vi } from "vitest";

import type { SyntaxWorkerRequest } from "../../src/syntax/SyntaxWorkerRequest";

const runtime = vi.hoisted(() => ({
    wasm: vi.fn(async () => undefined),
    append: vi.fn(async (_text: string, _hint: string | null, _signal: AbortSignal, _theme: string) => ({
        language: "rust",
        ranges: new Uint32Array()
    }))
}));
vi.mock("vscode-oniguruma", () => ({ loadWASM: runtime.wasm, OnigScanner: vi.fn(), OnigString: vi.fn() }));
vi.mock("../../src/syntax/assets", () => ({ loadSyntaxGrammar: vi.fn(), syntaxInjections: vi.fn() }));
vi.mock("../../src/syntax/SyntaxEngine", () => ({
    SyntaxEngine: class
    {
        append = runtime.append;
    }
}));

afterEach(() =>
{
    vi.unstubAllGlobals();
    vi.clearAllMocks();
    vi.resetModules();
});

it("acknowledges cancellation during WASM loading and reuses the in-flight initialization", async () =>
{
    let complete: ((response: Response) => void) | undefined;
    const fetch = vi.fn(async () =>
    {
        return new Promise<Response>(resolve =>
        {
            complete = resolve;
        });
    });
    const post = vi.fn();
    const worker = {
        onmessage: null as ((event: MessageEvent<SyntaxWorkerRequest>) => void) | null,
        postMessage: post
    };
    vi.stubGlobal("self", worker);
    vi.stubGlobal("fetch", fetch);
    await import("../../src/syntax/syntax.worker");
    const send = (data: SyntaxWorkerRequest): void => worker.onmessage?.({ data } as MessageEvent<SyntaxWorkerRequest>);
    send({
        kind: "tokenize",
        id: 1,
        text: "fn main() {}",
        language: "rust",
        theme: "light",
        continuation: { stream: 1, offset: 0, final: true }
    });
    send({ kind: "cancel", id: 1 });
    expect(post).toHaveBeenCalledWith({ kind: "cancelled", id: 1 }, { transfer: [] });
    for (let id = 10; id < 1010; id++)
    {
        send({
            kind: "tokenize",
            id,
            text: "fn obsolete() {}",
            language: "rust",
            theme: "light",
            continuation: { stream: 1, offset: 0, final: true }
        });
        send({ kind: "cancel", id });
        expect(post).toHaveBeenCalledWith({ kind: "cancelled", id }, { transfer: [] });
    }
    send({ kind: "detect", id: 2, text: "fn main() {}" });
    expect(post).toHaveBeenCalledWith({ kind: "detected", id: 2, language: "rust" }, { transfer: [] });
    send({
        kind: "tokenize",
        id: 3,
        text: "fn next() {}",
        language: "rust",
        theme: "light",
        continuation: { stream: 1, offset: 0, final: true }
    });
    complete?.(new Response(new ArrayBuffer(1)));
    await vi.waitFor(() =>
    {
        expect(runtime.append).toHaveBeenCalledTimes(1);
    });
    expect(fetch).toHaveBeenCalledTimes(1);
    expect(runtime.append.mock.calls[0]?.[0]).toBe("fn next() {}");
});
