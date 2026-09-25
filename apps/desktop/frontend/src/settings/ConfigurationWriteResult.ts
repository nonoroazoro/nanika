import type { SettingsWriteResult } from "./SettingsWriteResult";
import type { ConfigurationValue } from "../types/Settings";

export interface ConfigurationWriteResult extends SettingsWriteResult<Record<string, ConfigurationValue>>
{
    revision: number;
}
