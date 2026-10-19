import { DETAIL_TEXT_BATCH_CHARS, MAX_DETAIL_TEXT_CHUNKS } from "../generated/limits";

import type { TextChunk } from "../generated/TextChunk";
import type { TextDocument } from "../generated/TextDocument";
import type { TextRead } from "../generated/TextRead";

/**
 * An immutable, contiguous source. Reads have no dependency on view-event delivery.
 */
export class TextDocumentModel
{
    readonly document: TextDocument;
    private readonly _chunks: string[];
    private _length: number;
    private _pending: Promise<void> | null = null;
    private _disposed = false;
    private _epoch = 0;

    constructor(document: TextDocument, initial: string)
    {
        if (
            !Number.isSafeInteger(document.totalChunks) || document.totalChunks < 1
            || document.totalChunks > MAX_DETAIL_TEXT_CHUNKS
            || !Number.isSafeInteger(document.utf16Length) || document.utf16Length < 0
            || document.utf16Length > MAX_DETAIL_TEXT_CHUNKS * DETAIL_TEXT_BATCH_CHARS * 2
        )
        {
            throw new Error("Invalid text document metadata.");
        }
        this.document = document;
        this._chunks = [];
        this._length = 0;
        this._append({ documentId: document.id, revision: document.revision, index: 0, value: initial });
    }

    get chunks(): readonly string[]
    {
        return this._chunks;
    }

    get complete(): boolean
    {
        return this._chunks.length === this.document.totalChunks;
    }

    slice(start: number, end: number): string
    {
        const parts: string[] = [];
        let offset = 0;
        for (const chunk of this._chunks)
        {
            if (offset >= end)
            {
                break;
            }
            if (offset + chunk.length > start)
            {
                parts.push(chunk.slice(Math.max(0, start - offset), end - offset));
            }
            offset += chunk.length;
        }
        return parts.join("");
    }

    async readNext(read: (request: TextRead) => Promise<TextChunk>): Promise<void>
    {
        if (this._disposed)
        {
            throw new Error("Text source is no longer available.");
        }
        if (this.complete)
        {
            return Promise.resolve();
        }
        if (this._pending)
        {
            return this._pending;
        }
        const request = {
            documentId: this.document.id,
            revision: this.document.revision,
            index: this._chunks.length
        };
        const response = read(request);
        const epoch = this._epoch;
        this._pending = response.then(chunk =>
        {
            if (!this._disposed && epoch === this._epoch)
            {
                this._append(chunk);
            }
        }).finally(() =>
        {
            if (epoch === this._epoch)
            {
                this._pending = null;
            }
        });
        return this._pending;
    }

    cancelReads(): void
    {
        this._epoch++;
        this._pending = null;
    }

    dispose(): void
    {
        this.cancelReads();
        this._disposed = true;
        this._chunks.length = 0;
        this._length = 0;
    }

    private _append(chunk: TextChunk): void
    {
        const descriptor = this.document;
        let scalars = 0;
        for (let offset = 0; offset < chunk.value.length; offset++)
        {
            scalars++;
            if ((chunk.value.codePointAt(offset) ?? 0) > 0xffff)
            {
                offset++;
            }
        }
        const last = chunk.index === descriptor.totalChunks - 1;
        const length = this._length + chunk.value.length;
        if (
            chunk.documentId !== descriptor.id || chunk.revision !== descriptor.revision
            || chunk.index !== this._chunks.length || scalars > DETAIL_TEXT_BATCH_CHARS
            || (!last && scalars !== DETAIL_TEXT_BATCH_CHARS) || length > descriptor.utf16Length
            || (last && length !== descriptor.utf16Length)
        )
        {
            throw new Error("Text read does not match the immutable document.");
        }
        this._chunks.push(chunk.value);
        this._length = length;
    }
}
