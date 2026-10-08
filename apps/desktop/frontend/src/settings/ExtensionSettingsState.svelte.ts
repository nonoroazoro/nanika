import { SettingsState } from "./SettingsState.svelte";
import { prepareSetting } from "./validation";

import type { ConfigurationSaveOutcome } from "../generated/ConfigurationSaveOutcome";
import type { ExtensionSettings } from "../generated/ExtensionSettings";
import type { SaveSettingsRequest } from "../generated/SaveSettingsRequest";
import type { JsonValue } from "../generated/serde_json/JsonValue";

/**
 * Orders configuration facts across lifecycle snapshots and operation completions.
 * Operation errors remain attached to their operation even when its facts are older.
 */
export class ExtensionSettingsState extends SettingsState<Record<string, JsonValue>>
{
    private readonly _accept: (result: ConfigurationSaveOutcome) => ConfigurationSaveOutcome;

    constructor(
        configuration: NonNullable<ExtensionSettings["configuration"]>,
        error: string | null,
        write: (change: Omit<SaveSettingsRequest, "extensionId">) => Promise<ConfigurationSaveOutcome>,
        notify: (error: string) => void
    )
    {
        let latest: ConfigurationSaveOutcome = { ...configuration, error };
        const accept = (result: ConfigurationSaveOutcome): ConfigurationSaveOutcome =>
        {
            if (result.revision >= latest.revision)
            {
                latest = result;
            }
            return { ...latest, error: result.error };
        };
        super(
            latest,
            async change => accept(await write(change)),
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

    resumeConfiguration(key: string, completion: Promise<ConfigurationSaveOutcome>): void
    {
        this.resume(key, completion.then(result => this._accept(result)));
    }
}
