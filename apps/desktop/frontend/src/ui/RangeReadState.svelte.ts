/**
 * One bounded viewport read, with explicit recovery after a failed attempt.
 * Query changes invalidate completions; successful reads wait for delivered coverage.
 */
export class RangeReadState
{
    private _request = $state.raw<{ failed: boolean; key: string; pending: boolean; } | null>(null);

    get failed(): boolean
    {
        return this._request?.failed ?? false;
    }

    reset(): void
    {
        this._request = null;
    }

    async read(key: string, load: () => Promise<number | null>): Promise<void>
    {
        if (this._request?.pending || this._request?.key === key)
        {
            return;
        }
        const request = { key, pending: true, failed: false };
        this._request = request;
        let completed = false;
        try
        {
            completed = await load() !== null;
        }
        catch
        {
            // The transport owner reports the cause; this state owns retry eligibility.
        }
        if (this._request === request)
        {
            this._request = { key, pending: false, failed: !completed };
        }
    }
}
