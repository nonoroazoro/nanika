import type { SettingsSearchEntry } from "../types/SettingsSearchEntry";

/**
 * One in-flight search and one replaceable query. Results belong to a query revision,
 * including refreshes after lifecycle changes, so obsolete deliveries cannot replace the UI.
 */
export class SettingsSearchState
{
    query = $state("");
    results = $state.raw<SettingsSearchEntry[]>([]);
    busy = $state(false);
    hasCompletedSearch = $state(false);
    error = $state<string | null>(null);
    private _revision = 0;
    private _running = false;
    private _pending = false;
    private _visible = true;
    private readonly _search: (query: string) => Promise<SettingsSearchEntry[]>;

    constructor(search: (query: string) => Promise<SettingsSearchEntry[]>)
    {
        this._search = search;
    }

    update(query: string): void
    {
        this.query = query;
        this.refresh();
    }

    refresh(): void
    {
        this._revision++;
        this.error = null;
        this._pending = this._visible && this.query.trim().length > 0;
        // Match the root search presentation contract: keep the completed result window
        // while a replacement is pending. Only leaving search clears that window.
        if (!this._pending)
        {
            this.results = [];
            this.hasCompletedSearch = false;
        }
        this.busy = this._pending;
        if (!this._running && this._pending)
        {
            void this._run();
        }
    }

    setVisible(visible: boolean): void
    {
        if (this._visible !== visible)
        {
            this._visible = visible;
            this.refresh();
        }
    }

    private async _run(): Promise<void>
    {
        this._running = true;
        try
        {
            while (this._pending)
            {
                this._pending = false;
                const revision = this._revision;
                const query = this.query;
                try
                {
                    const results = await this._search(query);
                    if (revision === this._revision)
                    {
                        this.results = results;
                        this.hasCompletedSearch = true;
                    }
                }
                catch (error)
                {
                    if (revision === this._revision)
                    {
                        this.error = String(error);
                    }
                }
            }
        }
        finally
        {
            this._running = false;
            this.busy = false;
        }
    }
}
