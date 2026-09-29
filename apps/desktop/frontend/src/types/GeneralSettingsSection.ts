import type { GeneralSettingsField } from "./GeneralSettingsField";

export interface GeneralSettingsSection
{
    key: string;
    title: string;
    fields: GeneralSettingsField[];
}
