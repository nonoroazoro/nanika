import { SYNTAX_FINGERPRINT } from "./fingerprint";
import { VIEWPORT_LAYOUT_BYTES } from "./viewport-limits";

import type { SyntaxIdentity } from "./SyntaxIdentity";
import type { SyntaxProgress } from "./SyntaxProgress";

/**
 * Payload-bounded LRU. No DOM, pending work, or authority survives invalidation here.
 */
export class SyntaxCache
{
    private readonly _entries = new Map<
        string,
        { bytes: number; identity: SyntaxIdentity; snapshot: SyntaxProgress; }
    >();

    private _bytes = 0;

    private readonly _maximumBytes: number;
    private readonly _maximumEntries: number;

    constructor(maximumBytes = 8 * 1024 * 1024, maximumEntries = 128)
    {
        this._maximumBytes = maximumBytes;
        this._maximumEntries = maximumEntries;
    }

    get bytes(): number
    {
        return this._bytes;
    }

    get size(): number
    {
        return this._entries.size;
    }

    get(identity: SyntaxIdentity): SyntaxProgress | null
    {
        const key = syntaxKey(identity);
        const entry = this._entries.get(key);
        if (!entry)
        {
            return null;
        }
        this._entries.delete(key);
        this._entries.set(key, entry);
        return entry.snapshot;
    }

    put(identity: SyntaxIdentity, snapshot: SyntaxProgress): void
    {
        this.invalidate(identity.scope, identity.id, identity.revision);
        this._remove(syntaxKey(identity));
        const key = syntaxKey(identity);
        const bytes = snapshot.result.ranges.byteLength + (key.length * 2) + 128
            + (snapshot.result.ranges.length ? VIEWPORT_LAYOUT_BYTES : 0);
        if (bytes > this._maximumBytes)
        {
            return;
        }
        this._entries.set(key, { identity, snapshot, bytes });
        this._bytes += bytes;
        while (this._bytes > this._maximumBytes || this._entries.size > this._maximumEntries)
        {
            const oldest = this._entries.keys().next().value;
            if (oldest === undefined)
            {
                break;
            }
            this._remove(oldest);
        }
    }

    invalidate(scope: string, id?: string, keepRevision?: string): void
    {
        for (const [key, entry] of this._entries)
        {
            if (
                entry.identity.scope === scope && (id === undefined || entry.identity.id === id)
                && (keepRevision === undefined || entry.identity.revision !== keepRevision)
            )
            {
                this._remove(key);
            }
        }
    }

    clear(): void
    {
        this._entries.clear();
        this._bytes = 0;
    }

    private _remove(key: string): void
    {
        const entry = this._entries.get(key);
        if (entry)
        {
            this._bytes -= entry.bytes;
            this._entries.delete(key);
        }
    }
}

/**
 * The assets and palette are build-time inputs. A frontend reload revokes this cache.
 *
 * @param identity Immutable document authority.
 */
export function syntaxKey(identity: SyntaxIdentity): string
{
    return JSON.stringify([
        SYNTAX_FINGERPRINT,
        identity.scope,
        identity.id,
        identity.revision,
        identity.theme
    ]);
}
