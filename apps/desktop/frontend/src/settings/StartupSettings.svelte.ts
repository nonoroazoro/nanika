import { SettingsState } from "./SettingsState.svelte";

import type { SettingsWriteResult } from "./SettingsWriteResult";
import type { StartupStatus } from "../types/Settings";

/**
 * Keeps native startup status current without replacing edits or racing OS writes.
 */
export class StartupSettings
{
    status: StartupStatus | null = $state(null);
    readonly settings: SettingsState<{ enabled: boolean; }>;
    private readonly _read: () => Promise<StartupStatus>;

    constructor(
        read: () => Promise<StartupStatus>,
        write: (enabled: boolean) => Promise<StartupStatus>,
        notify: (error: string) => void
    )
    {
        this._read = read;
        const initial = { enabled: false };
        this.settings = new SettingsState(
            { values: initial, saved: initial, effective: null, error: null },
            async (_key, enabled) =>
            {
                let status: StartupStatus;
                let error: string | null = null;
                try
                {
                    status = await write(enabled);
                }
                catch (failure)
                {
                    error = String(failure);
                    status = await read();
                }
                return this._result(status, error);
            },
            undefined,
            notify
        );
    }

    async refresh(): Promise<void>
    {
        return this.settings.refresh(async () => this._result(await this._read(), null));
    }

    private _result(status: StartupStatus, error: string | null): SettingsWriteResult<{ enabled: boolean; }>
    {
        this.status = status;
        const values = { enabled: status === "enabled" };
        return { values, saved: values, effective: values, error };
    }
}
