import { LANGUAGE_SCOPES } from "./languages";
import { DETECTION_LIMIT, LINE_LENGTH_LIMIT, RANGE_LIMIT, TOKEN_BATCH_CHARS } from "./limits";
import { TOKEN_COLORS } from "./palette";
import { SyntaxCache, syntaxKey } from "./SyntaxCache";

import type { SyntaxContinuation } from "./SyntaxContinuation";
import type { SyntaxIdentity } from "./SyntaxIdentity";
import type { SyntaxJob } from "./SyntaxJob";
import type { SyntaxLanguage } from "./SyntaxLanguage";
import type { SyntaxProgress } from "./SyntaxProgress";
import type { SyntaxResult } from "./SyntaxResult";
import type { SyntaxWorkerRequest } from "./SyntaxWorkerRequest";
import type { SyntaxWorkerResponse } from "./SyntaxWorkerResponse";

/**
 * One lazy engine, bounded pending demand, synchronous resident lookup.
 * Consumers retain the service for their mounted lifetime and abort obsolete requests.
 */
export class SyntaxService
{
    readonly cache = new SyntaxCache();
    private readonly _invalidated = new Set<(scope: string, id?: string, keepRevision?: string) => void>();
    private readonly _queue: SyntaxJob[] = [];
    private _active: SyntaxJob | null = null;
    private _worker: Worker | null = null;
    private _ready = false;
    private _tokenizing = false;
    private _sequence = 0;
    private _leases = 0;
    private _failures = 0;
    private _watchdog: ReturnType<typeof setTimeout> | null = null;

    private readonly _createWorker: () => Worker;

    constructor(
        createWorker: () => Worker = () =>
            new Worker(new URL("./syntax.worker.ts", import.meta.url), { type: "module" })
    )
    {
        this._createWorker = createWorker;
    }

    retain(): () => void
    {
        // A new visible consumer session may recover, but never retries abandoned work.
        if (this._leases === 0)
        {
            this._failures = 0;
        }
        this._leases++;
        let released = false;
        return () =>
        {
            if (!released)
            {
                released = true;
                if (--this._leases === 0)
                {
                    this.suspend();
                }
            }
        };
    }

    onInvalidation(listener: (scope: string, id?: string, keepRevision?: string) => void): () => void
    {
        this._invalidated.add(listener);
        return () =>
        {
            this._invalidated.delete(listener);
        };
    }

    lookup(identity: SyntaxIdentity): SyntaxProgress | null
    {
        return this.cache.get(identity);
    }

    /**
     * Classify a bounded prefix before loading a grammar.
     * The shared queue and AbortSignal revoke detection just like tokenization.
     */
    async detect(identity: SyntaxIdentity, prefix: string, signal: AbortSignal): Promise<SyntaxLanguage | null>
    {
        const result = await this._request(
            {
                kind: "detect",
                id: ++this._sequence,
                text: prefix.slice(0, DETECTION_LIMIT)
            },
            identity,
            signal
        );
        return typeof result === "string" ? result : null;
    }

    openStream(): number
    {
        return ++this._sequence;
    }

    closeStream(stream: number): void
    {
        if (this._worker)
        {
            this._send({ kind: "release", stream });
        }
    }

    async append(
        identity: SyntaxIdentity,
        text: string,
        language: SyntaxLanguage,
        signal: AbortSignal,
        continuation: SyntaxContinuation
    ): Promise<SyntaxResult | null>
    {
        if (text.length > TOKEN_BATCH_CHARS)
        {
            return null;
        }
        const result = await this._request(
            {
                kind: "tokenize",
                id: ++this._sequence,
                text,
                language,
                continuation,
                theme: identity.theme
            },
            identity,
            signal
        );
        return typeof result === "object" ? result : null;
    }

    invalidate(scope: string, id?: string, keepRevision?: string): void
    {
        this.cache.invalidate(scope, id, keepRevision);
        for (const listener of this._invalidated)
        {
            listener(scope, id, keepRevision);
        }
        const matches = (job: SyntaxJob): boolean =>
            job.identity.scope === scope
            && (id === undefined || job.identity.id === id)
            && (keepRevision === undefined || job.identity.revision !== keepRevision);
        for (let index = this._queue.length - 1; index >= 0; index--)
        {
            const job = this._queue[index];
            if (job && matches(job))
            {
                this._queue.splice(index, 1);
                this._finish(job, null);
            }
        }
        if (this._active && matches(this._active))
        {
            this._finish(this._active, null);
            this._arm(150, () =>
            {
                this._stop();
            });
            this._send({ kind: "cancel", id: this._active.id });
        }
    }

    suspend(): void
    {
        for (const job of this._queue.splice(0))
        {
            this._finish(job, null);
        }
        this._stop(false);
    }

    private async _request(
        request: Extract<SyntaxWorkerRequest, { kind: "detect" | "tokenize"; }>,
        identity: SyntaxIdentity,
        signal: AbortSignal
    ): Promise<SyntaxLanguage | SyntaxResult | null>
    {
        if (signal.aborted || this._failures >= 2)
        {
            return null;
        }
        const continuation = request.kind === "tokenize" ? request.continuation : null;
        const key = `${request.kind}:${syntaxKey(identity)}:${continuation?.stream ?? ""}:${
            continuation?.offset ?? ""
        }`;
        let job = this._active?.key === key && this._active.listeners.size
            ? this._active
            : this._queue.find(item => item.key === key);
        if (!job)
        {
            job = { ...request, key, identity, listeners: new Set() };
            if (this._queue.length >= 8)
            {
                const discarded = this._queue.shift();
                if (discarded)
                {
                    this._finish(discarded, null);
                }
            }
            this._queue.push(job);
        }
        const demand = job;
        return new Promise(resolve =>
        {
            const callbacks = {
                finish: (snapshot: SyntaxLanguage | SyntaxResult | null): void =>
                {
                    signal.removeEventListener("abort", callbacks.cancel);
                    demand.listeners.delete(callbacks.finish);
                    resolve(snapshot);
                },
                cancel: (): void =>
                {
                    callbacks.finish(null);
                    if (demand.listeners.size === 0)
                    {
                        if (this._active === demand)
                        {
                            // Cancellation must acknowledge promptly even if one regex call never returns.
                            this._arm(150, () =>
                            {
                                this._stop();
                            });
                            this._send({ kind: "cancel", id: demand.id });
                        }
                        else
                        {
                            const index = this._queue.indexOf(demand);
                            if (index >= 0)
                            {
                                this._queue.splice(index, 1);
                            }
                        }
                    }
                }
            };
            demand.listeners.add(callbacks.finish);
            signal.addEventListener("abort", callbacks.cancel, { once: true });
            this._pump();
        });
    }

    private _pump(): void
    {
        if (!this._queue.length || this._active || this._failures >= 2)
        {
            return;
        }
        if (!this._worker)
        {
            try
            {
                const worker = this._createWorker();
                this._worker = worker;
                worker.onmessage = (event: MessageEvent<SyntaxWorkerResponse>) =>
                {
                    if (worker === this._worker)
                    {
                        this._receive(event.data);
                    }
                };
                const failed = (): void =>
                {
                    if (worker === this._worker)
                    {
                        this._fail();
                    }
                };
                worker.onerror = failed;
                worker.onmessageerror = failed;
                this._arm(5_000, () =>
                {
                    this._fail();
                });
            }
            catch
            {
                this._fail();
            }
            return;
        }
        if (!this._ready)
        {
            return;
        }
        const job = this._queue.shift();
        if (!job)
        {
            return;
        }
        this._active = job;
        this._tokenizing = false;
        if (job.kind === "detect")
        {
            // Detection has bounded input and no grammar work or whole-job deadline.
            // Cancellation still terminates an unresponsive worker through the shared path.
            this._send({ kind: "detect", id: job.id, text: job.text });
        }
        else
        {
            // WASM initialization has its own budget; tokenizing starts the job deadline.
            this._arm(5_000, () =>
            {
                this._fail();
            });
            this._send({
                kind: "tokenize",
                id: job.id,
                language: job.language,
                text: job.text,
                theme: job.identity.theme,
                continuation: job.continuation
            });
        }
    }

    private _receive(message: unknown): void
    {
        if (!message || typeof message !== "object" || !("kind" in message))
        {
            this._fail();
            return;
        }
        if (message.kind === "ready")
        {
            if (!this._ready)
            {
                this._clearWatchdog();
                this._ready = true;
                this._pump();
            }
            return;
        }
        if (!("id" in message))
        {
            this._fail();
            return;
        }
        if (message.kind === "error" && message.id === null)
        {
            this._fail();
            return;
        }
        const job = this._active;
        if (!job || message.id !== job.id)
        {
            return;
        }
        if (message.kind === "tokenizing")
        {
            // A late start must not replace the shorter cancellation watchdog.
            if (job.kind === "tokenize" && job.listeners.size && !this._tokenizing)
            {
                this._tokenizing = true;
                this._arm(1_500, () =>
                {
                    this._fail();
                });
            }
            return;
        }
        this._clearWatchdog();
        this._active = null;
        let snapshot: SyntaxLanguage | SyntaxResult | null = null;
        if (
            job.kind === "tokenize" && message.kind === "result" && job.listeners.size && "result" in message
            && this._validResult(
                message.result,
                job.continuation.offset + job.text.length,
                Math.max(0, job.continuation.offset - LINE_LENGTH_LIMIT)
            )
            && message.result.language === job.language
        )
        {
            this._failures = 0;
            snapshot = message.result;
        }
        else if (
            job.kind === "detect" && message.kind === "detected" && job.listeners.size && "language" in message
            && (message.language === null
                || (typeof message.language === "string" && Object.hasOwn(LANGUAGE_SCOPES, message.language)))
        )
        {
            snapshot = message.language as SyntaxLanguage | null;
        }
        this._finish(job, snapshot);
        this._pump();
    }

    private _validResult(result: unknown, length: number, minimum: number): result is SyntaxResult
    {
        if (
            !result || typeof result !== "object" || !("ranges" in result)
            || !("language" in result)
        )
        {
            return false;
        }
        const { ranges, language } = result;
        if (
            !(ranges instanceof Uint32Array) || ranges.length % 3 !== 0 || ranges.length > RANGE_LIMIT * 3
            || typeof language !== "string" || !Object.hasOwn(LANGUAGE_SCOPES, language)
        )
        {
            return false;
        }
        let end = minimum;
        for (let index = 0; index < ranges.length; index += 3)
        {
            const start = ranges[index] ?? 0;
            const stop = ranges[index + 1] ?? 0;
            const color = ranges[index + 2] ?? 0;
            if (start < end || stop <= start || stop > length || color < 1 || color >= TOKEN_COLORS.length)
            {
                return false;
            }
            end = stop;
        }
        return true;
    }

    private _send(message: SyntaxWorkerRequest): void
    {
        try
        {
            this._worker?.postMessage(message);
        }
        catch
        {
            // Delivery failures follow the same bounded lifecycle as asynchronous Worker errors.
            this._fail();
        }
    }

    private _finish(job: SyntaxJob, snapshot: SyntaxLanguage | SyntaxResult | null): void
    {
        for (const listener of [...job.listeners])
        {
            listener(snapshot);
        }
    }

    private _arm(milliseconds: number, action: () => void): void
    {
        this._clearWatchdog();
        this._watchdog = setTimeout(action, milliseconds);
    }

    private _clearWatchdog(): void
    {
        if (this._watchdog !== null)
        {
            clearTimeout(this._watchdog);
            this._watchdog = null;
        }
    }

    private _fail(): void
    {
        if (++this._failures === 1)
        {
            console.error("Syntax highlighting is unavailable for this request.");
        }
        // Never retry failed work automatically or log source contents.
        this.suspend();
    }

    private _stop(resume = true): void
    {
        this._clearWatchdog();
        if (this._worker)
        {
            this._worker.onmessage = null;
            this._worker.onerror = null;
            this._worker.onmessageerror = null;
            this._worker.terminate();
        }
        this._worker = null;
        this._ready = false;
        if (this._active)
        {
            this._finish(this._active, null);
            this._active = null;
        }
        if (resume)
        {
            this._pump();
        }
    }
}
