import { SettingsState } from "./SettingsState.svelte";
import { prepareSetting } from "./validation";

import type { ConfigurationWriteResult } from "./ConfigurationWriteResult";
import type { ConfigurationValue, ExtensionSettings } from "../types/Settings";

/**
 * Orders configuration facts across lifecycle snapshots and operation completions.
 * Operation errors remain attached to their operation even when its facts are older.
 */
export class ExtensionSettingsState extends SettingsState<Record<string, ConfigurationValue>>
{
    private readonly _accept: (result: ConfigurationWriteResult) => ConfigurationWriteResult;

    constructor(
        configuration: NonNullable<ExtensionSettings["configuration"]>,
        error: string | null,
        write: (key: string, value: ConfigurationValue) => Promise<ConfigurationWriteResult>,
        notify: (error: string) => void
    )
    {
        let latest: ConfigurationWriteResult = { ...configuration, error };
        const accept = (result: ConfigurationWriteResult): ConfigurationWriteResult =>
        {
            if (result.revision >= latest.revision)
            {
                latest = result;
            }
            return { ...latest, error: result.error };
        };
        super(
            latest,
            async (key, value) => accept(await write(key, value)),
            (key, draft) => prepareSetting(configuration.contribution.properties, key, draft),
            notify
        );
        this._accept = accept;
    }

    observeConfiguration(configuration: NonNullable<ExtensionSettings["configuration"]>): void
    {
        this._accept({ ...configuration, error: null });
        this.observe(() => this._accept({ ...configuration, error: null }));
    }

    resumeConfiguration(key: string, completion: Promise<ConfigurationWriteResult>): void
    {
        this.resume(key, completion.then(result => this._accept(result)));
    }
}
