import { loadWASM, OnigScanner, OnigString } from "vscode-oniguruma";
import wasmUrl from "vscode-oniguruma/release/onig.wasm?url";

import { loadSyntaxGrammar, syntaxInjections } from "./assets";
import { detectSyntax } from "./detect";
import { SyntaxEngine } from "./SyntaxEngine";

import type { SyntaxWorkerRequest } from "./SyntaxWorkerRequest";
import type { SyntaxWorkerResponse } from "./SyntaxWorkerResponse";

let active: { controller: AbortController; request: Extract<SyntaxWorkerRequest, { kind: "tokenize"; }>; } | null =
    null;
let engine: SyntaxEngine | null = null;
let initializing = false;

self.onmessage = (event: MessageEvent<SyntaxWorkerRequest>): void =>
{
    const request = event.data;
    if (request.kind === "release")
    {
        if (active?.request.continuation.stream === request.stream)
        {
            _cancel();
        }
        engine?.release(request.stream);
        return;
    }
    if (request.kind === "cancel")
    {
        if (active?.request.id === request.id)
        {
            _cancel();
        }
        return;
    }
    if (request.kind === "detect")
    {
        _send({ kind: "detected", id: request.id, language: detectSyntax(request.text) });
        return;
    }
    if (active)
    {
        _send({ kind: "error", id: request.id });
        return;
    }
    active = { request, controller: new AbortController() };
    if (engine)
    {
        void _tokenize(active, engine);
    }
    else if (!initializing)
    {
        initializing = true;
        void _initialize().catch(() =>
        {
            _send({ kind: "error", id: null });
        });
    }
};

_send({ kind: "ready" });

function _cancel(): void
{
    if (!active)
    {
        return;
    }
    active.controller.abort();
    if (!engine)
    {
        // Initialization belongs to the worker; abandoned source can be released now.
        const id = active.request.id;
        active = null;
        _send({ kind: "cancelled", id });
    }
}

async function _initialize(): Promise<void>
{
    const response = await fetch(wasmUrl);
    if (!response.ok)
    {
        throw new Error("Syntax runtime could not be loaded.");
    }
    await loadWASM(await response.arrayBuffer());
    engine = new SyntaxEngine({
        onigLib: Promise.resolve({
            createOnigScanner: patterns => new OnigScanner(patterns),
            createOnigString: value => new OnigString(value)
        }),
        loadGrammar: loadSyntaxGrammar,
        getInjections: syntaxInjections
    });
    if (active)
    {
        void _tokenize(active, engine);
    }
}

async function _tokenize(job: NonNullable<typeof active>, tokenizer: SyntaxEngine): Promise<void>
{
    const { request, controller } = job;
    _send({ kind: "tokenizing", id: request.id });
    try
    {
        const result = await tokenizer.append(
            request.text,
            request.language,
            controller.signal,
            request.theme,
            request.continuation
        );
        controller.signal.throwIfAborted();
        _send({ kind: "result", id: request.id, result }, [result.ranges.buffer as ArrayBuffer]);
    }
    catch
    {
        _send({ kind: controller.signal.aborted ? "cancelled" : "error", id: request.id });
    }
    finally
    {
        active = null;
    }
}

function _send(message: SyntaxWorkerResponse, transfer: Transferable[] = []): void
{
    self.postMessage(message, { transfer });
}
