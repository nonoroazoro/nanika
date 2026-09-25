import type { ContributionIcon } from "./ContributionIcon";
import type { OperationProgress } from "./OperationProgress";
import type { SettingsWriteResult } from "../settings/SettingsWriteResult";

export type ConfigurationValue =
    | {
        [key: string]: ConfigurationValue;
    }
    | boolean
    | ConfigurationValue[]
    | number
    | string
    | null;

export interface ConfigurationSchema
{
    type: "array" | "boolean" | "integer" | "object" | "string";
    allowUnlimited?: boolean;
    default?: ConfigurationValue;
    format?: "directory" | "path" | null;
    minimum?: number | null;
    maximum?: number | null;
    multipleOf?: number | null;
    maxLength?: number | null;
    maxItems?: number | null;
    items?: ConfigurationSchema | null;
    properties: Record<string, ConfigurationSchema>;
    required: string[];
}

export interface ConfigurationProperty extends ConfigurationSchema
{
    title: string;
    persistence: "afterApply" | "beforeApply";
    description: string | null;
    default: ConfigurationValue;
    platforms?: Array<"macos" | "windows">;
    order?: number;
}

export interface ExtensionSettings
{
    id: string;
    name: string;
    icon: ContributionIcon;
    enabled: boolean;
    configurationError: string | null;
    application: SettingsApplicationUpdate | null;
    configuration: {
        contribution: { properties: Record<string, ConfigurationProperty>; title: string; };
        effective: Record<string, ConfigurationValue> | null;
        extensionId: string;
        saved: Record<string, ConfigurationValue>;
        values: Record<string, ConfigurationValue>;
    } | null;
}

export interface SettingsSnapshot
{
    maximized: boolean;
    version: string;
    general: HostPreferences;
    extensions: ExtensionSettings[];
}

export interface HostPreferences
{
    formatVersion: 1;
    launcherShortcut: string;
    theme: "dark" | "light" | "system";
    hideOnBlur: boolean;
}

export type StartupStatus = "disabled" | "enabled" | "needsRepair" | "notFound" | "requiresApproval";

export type SettingsSaveResult =
    | { error: string; status: "failed"; }
    | { progress: OperationProgress | null; status: "running"; }
    | ({ status: "completed"; } & SettingsWriteResult<Record<string, ConfigurationValue>>);

export interface SettingsApplicationUpdate
{
    requestId: number;
    extensionId: string;
    key: string;
    result: SettingsSaveResult;
}
