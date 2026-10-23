import { SettingsState } from "./SettingsState.svelte";
import { operationFailure } from "../localization/operation-errors";

import type { SettingsError } from "./SettingsError";
import type { SettingsResult } from "./SettingsResult";
import type { OperationError } from "../generated/OperationError";
import type { StartupStatus } from "../generated/StartupStatus";

/**
 * Keeps native startup status current without replacing edits or racing OS writes.
 */
export class StartupSettings
{
    status: StartupStatus | null = $state(null);
    readError: OperationError | null = $state(null);
    reading = $state(false);
    readonly settings: SettingsState<{ enabled: boolean; }>;
    private readonly _read: () => Promise<StartupStatus>;

    constructor(
        read: () => Promise<StartupStatus>,
        write: (enabled: boolean) => Promise<StartupStatus>,
        notify: (error: SettingsError) => void
    )
    {
        this._read = read;
        const initial = { enabled: false };
        this.settings = new SettingsState(
            { values: initial, saved: initial, effective: null, error: null },
            async ({ value: enabled }) =>
            {
                try
                {
                    const status = await write(enabled);
                    this.readError = null;
                    return this._result(status, null);
                }
                catch (failure)
                {
                    const error = operationFailure(failure, "startupWriteUnconfirmed");
                    try
                    {
                        return this._result(await this._readStatus(), error);
                    }
                    catch
                    {
                        // Keep both failures: the field owns the write outcome and
                        // readError explains why the current OS state is still unknown.
                        throw Object.assign(new Error("Startup change could not be confirmed."), error);
                    }
                }
            },
            undefined,
            notify
        );
    }

    async refresh(): Promise<void>
    {
        return this.settings.refresh(async () => this._result(await this._readStatus(), null));
    }

    private async _readStatus(): Promise<StartupStatus>
    {
        this.reading = true;
        try
        {
            const status = await this._read();
            this.readError = null;
            return status;
        }
        catch (error)
        {
            this.readError = operationFailure(error, "startupReadFailed");
            throw Object.assign(new Error("Startup status could not be read."), this.readError);
        }
        finally
        {
            this.reading = false;
        }
    }

    private _result(status: StartupStatus, error: OperationError | null): SettingsResult<{ enabled: boolean; }>
    {
        this.status = status;
        const values = { enabled: status === "enabled" };
        return { values, saved: values, effective: values, error };
    }
}
