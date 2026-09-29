import type { SettingsSearchEntry } from "./SettingsSearchEntry";

export interface SettingsSearchGroup
{
    pageId: string;
    title: string;
    entries: SettingsSearchEntry[];
}
