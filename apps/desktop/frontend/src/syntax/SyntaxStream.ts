import { LINE_LENGTH_LIMIT, TOKEN_BATCH_CHARS } from "./limits";

import type { SyntaxIdentity } from "./SyntaxIdentity";
import type { SyntaxLanguage } from "./SyntaxLanguage";
import type { SyntaxProgress } from "./SyntaxProgress";
import type { SyntaxService } from "./SyntaxService";
import type { TextDocumentModel } from "../text/TextDocumentModel";

/**
 * A visible preview advances one sequential grammar cursor only as far as requested.
 * Source reads remain owned by the existing scroll loader.
 */
export class SyntaxStream
{
    private readonly _controller = new AbortController();
    private readonly _id: number;
    private readonly _residentEnd: number;
    private _target = 0;
    private _offset = 0;
    private _running = false;
    private _language: SyntaxLanguage | null = null;
    private _tokens = new Uint32Array(0);
    private _length = 0;
    private readonly _service: SyntaxService;
    private readonly _identity: SyntaxIdentity;
    private readonly _source: TextDocumentModel;
    private readonly _publish: (progress: SyntaxProgress) => void;

    constructor(
        service: SyntaxService,
        identity: SyntaxIdentity,
        source: TextDocumentModel,
        publish: (progress: SyntaxProgress) => void
    )
    {
        this._service = service;
        this._identity = identity;
        this._source = source;
        this._publish = publish;
        this._id = service.openStream();
        const resident = service.lookup(identity);
        this._residentEnd = resident?.processed ?? 0;
        this._language = resident?.result.language ?? null;
    }

    advance(end: number): void
    {
        if (this._controller.signal.aborted || this._offset >= this._source.document.utf16Length)
        {
            return;
        }
        if (end <= 0)
        {
            this._target = 0;
            return;
        }
        // Finish the visible logical line when it is already resident, without
        // reading ahead or scanning an unbounded minified line.
        const tail = this._source.slice(end, end + LINE_LENGTH_LIMIT);
        const newline = tail.indexOf("\n");
        let boundary = end;
        if (newline >= 0)
        {
            boundary += newline + 1;
        }
        else if (end + tail.length === this._source.document.utf16Length)
        {
            boundary += tail.length;
        }
        this._target = Math.min(
            this._source.document.utf16Length,
            Math.ceil(boundary / TOKEN_BATCH_CHARS) * TOKEN_BATCH_CHARS
        );
        // Cached colors need no worker until scrolling crosses their boundary.
        // A new cursor then replays the prefix to reconstruct cross-line state.
        if (this._target <= this._residentEnd)
        {
            return;
        }
        if (!this._running && this._offset < this._target)
        {
            this._running = true;
            void this._run();
        }
    }

    dispose(): void
    {
        if (this._controller.signal.aborted)
        {
            return;
        }
        this._controller.abort();
        this._service.closeStream(this._id);
        this._tokens = new Uint32Array(0);
        this._length = 0;
    }

    private async _run(): Promise<void>
    {
        const signal = this._controller.signal;
        try
        {
            const language = this._language
                ?? await this._service.detect(this._identity, this._source.chunks[0] ?? "", signal);
            this._language = language;
            if (!language)
            {
                this.dispose();
                return;
            }
            while (!signal.aborted && this._offset < this._target)
            {
                const text = this._source.slice(this._offset, this._offset + TOKEN_BATCH_CHARS);
                if (!text.length)
                {
                    break;
                }
                const next = this._offset + text.length;
                const batch = await this._service.append(this._identity, text, language, signal, {
                    stream: this._id,
                    offset: this._offset,
                    final: next === this._source.document.utf16Length
                });
                if (signal.aborted)
                {
                    return;
                }
                if (!batch)
                {
                    this.dispose();
                    return;
                }
                this._offset = next;
                this._append(batch.ranges);
                const result = { ...batch, ranges: this._tokens.subarray(0, this._length) };
                const prefix = this._source.chunks[0] ?? "";
                let progress: SyntaxProgress = { processed: next, result };
                // Keep a bounded first-screen restore; the worker cursor belongs only
                // to this mounted preview and is never resurrected from a color cache.
                if (next <= prefix.length && next > (this._service.lookup(this._identity)?.processed ?? 0))
                {
                    const snapshot = {
                        processed: next,
                        result: { ...result, ranges: result.ranges.slice() }
                    };
                    this._service.cache.put(this._identity, snapshot);
                    progress = snapshot;
                }
                this._publish(progress);
            }
        }
        catch
        {
            this.dispose();
        }
        finally
        {
            this._running = false;
        }
    }

    private _append(ranges: Uint32Array): void
    {
        const needed = this._length + ranges.length;
        if (needed > this._tokens.length)
        {
            const expanded = new Uint32Array(Math.max(needed, 1024, this._tokens.length * 2));
            expanded.set(this._tokens.subarray(0, this._length));
            this._tokens = expanded;
        }
        this._tokens.set(ranges, this._length);
        this._length = needed;
    }
}
