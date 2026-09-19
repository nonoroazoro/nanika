import type { ContributionIcon } from "./ContributionIcon";

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
    description: string | null;
    default: ConfigurationValue;
}

export interface ExtensionSettings
{
    id: string;
    name: string;
    icon: ContributionIcon;
    application: SettingsApplicationUpdate | null;
    configuration: {
        contribution: { properties: Record<string, ConfigurationProperty>; title: string; };
        extensionId: string;
        values: Record<string, ConfigurationValue>;
    };
}

export interface SettingsSnapshot
{
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

export interface SettingsSaveResult
{
    status: "applied" | "applyFailed" | "applying" | "nextLaunch";
    error: string | null;
}

export interface SettingsApplicationUpdate
{
    requestId: number;
    extensionId: string;
    result: SettingsSaveResult;
}
