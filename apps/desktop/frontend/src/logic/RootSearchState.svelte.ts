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
    private _confirmation = $state<string | null>(null);

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

    get confirmationTitle(): string | null
    {
        const selected = this.selectedResult;
        return selected && this._confirmation === this._confirmationKey(selected)
            ? selected.confirmationTitle
            : null;
    }

    static identity(result: SearchResult): string
    {
        return JSON.stringify([result.extensionId, result.entryId, result.actionId]);
    }

    /**
     * Resolves direct activation against the displayed result revision.
     * @param result The visible result the user activated.
     */
    activate(result: SearchResult): "confirmed" | "default" | null
    {
        if (this.snapshot.phase !== "ready" || !this.snapshot.results.includes(result))
        {
            this.cancelConfirmation();
            return null;
        }
        if (result.allowDefaultExecution)
        {
            this.cancelConfirmation();
            return "default";
        }
        if (!result.confirmationTitle)
        {
            this.cancelConfirmation();
            return null;
        }
        this.select(this.snapshot.resultOffset + this.snapshot.results.indexOf(result));
        const key = this._confirmationKey(result);
        if (this._confirmation === key)
        {
            this.cancelConfirmation();
            return "confirmed";
        }
        this._confirmation = key;
        return null;
    }

    cancelConfirmation(): void
    {
        this._confirmation = null;
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
            this.cancelConfirmation();
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
            this.cancelConfirmation();
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
        // A page or payload replacement must not resurrect a previously reviewed target.
        if (this._confirmation !== null && this.confirmationTitle === null)
        {
            this.cancelConfirmation();
        }
    }

    select(index: number): void
    {
        const nextIndex = clampIndex(index, this.snapshot.totalResults);
        if (nextIndex !== this._selectedIndex)
        {
            this.cancelConfirmation();
        }
        this._selectedIndex = nextIndex;
        const result = this.snapshot.results[this._selectedIndex - this.snapshot.resultOffset];
        this._selectedId = result ? RootSearchState.identity(result) : null;
    }

    private _confirmationKey(result: SearchResult): string
    {
        return JSON.stringify([
            this.snapshot.sessionId,
            this.snapshot.requestId,
            this.snapshot.resultRevision,
            RootSearchState.identity(result),
            result.title,
            result.confirmationTitle
        ]);
    }
}
