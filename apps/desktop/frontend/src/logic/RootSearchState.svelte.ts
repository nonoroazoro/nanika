import { clampIndex } from "./clampIndex";

import type { RootSearchSnapshot, SearchResult } from "../types";

/**
 * Keeps the displayed result window and selection coherent across Channel updates.
 * Pending queries retain the previous window until a completed ranking arrives.
 */
export class RootSearchState
{
    scrollTop = $state(0);
    private _snapshot: RootSearchSnapshot;
    private _resultRequestId: number | null = null;
    private _selectedIndex = $state(-1);
    private _selectedId = $state<string | null>(null);

    constructor(snapshot: RootSearchSnapshot)
    {
        this._snapshot = $state.raw(snapshot);
        if (snapshot.phase === "ready")
        {
            this._resultRequestId = snapshot.requestId;
            this.select(snapshot.resultOffset);
        }
    }

    get snapshot(): RootSearchSnapshot
    {
        return this._snapshot;
    }

    get selectedIndex(): number
    {
        return this._selectedIndex;
    }

    get selectedResult(): SearchResult | null
    {
        const result = this.snapshot.results[this._selectedIndex - this.snapshot.resultOffset];
        return result && (this._selectedId === null || RootSearchState.identity(result) === this._selectedId)
            ? result
            : null;
    }

    static identity(result: SearchResult): string
    {
        return JSON.stringify([result.extensionId, result.entryId, result.actionId]);
    }

    /**
     * Accepts an already ordered and session-authorized Channel update.
     * A page change cannot prove that an offscreen selection was removed.
     * @param next The snapshot validated by the session owner.
     */
    accept(next: RootSearchSnapshot): void
    {
        const previous = this.snapshot;
        if (next.phase === "searching")
        {
            this._snapshot = {
                ...next,
                results: previous.results,
                resultRevision: previous.resultRevision,
                resultOffset: previous.resultOffset,
                totalResults: previous.totalResults
            };
            return;
        }
        const queryChanged = this._resultRequestId !== next.requestId;
        const rankingChanged = queryChanged || previous.resultRevision !== next.resultRevision;
        this._snapshot = next;
        this._resultRequestId = next.requestId;
        if (queryChanged)
        {
            this.scrollTop = 0;
        }
        if (rankingChanged)
        {
            const matched = queryChanged
                ? -1
                : next.results.findIndex(result => RootSearchState.identity(result) === this._selectedId);
            // Preserve identity within the delivered window. Otherwise choose its nearest
            // available row, so a replacement ranking always has an actionable selection.
            const local = matched >= 0
                ? matched
                : clampIndex(queryChanged ? 0 : this._selectedIndex - next.resultOffset, next.results.length);
            this.select(local < 0 ? -1 : next.resultOffset + local);
        }
        else if (this._selectedId === null)
        {
            // Keyboard navigation can select an index before its requested page arrives.
            this.select(this._selectedIndex);
        }
    }

    select(index: number): void
    {
        this._selectedIndex = clampIndex(index, this.snapshot.totalResults);
        const result = this.snapshot.results[this._selectedIndex - this.snapshot.resultOffset];
        this._selectedId = result ? RootSearchState.identity(result) : null;
    }
}
