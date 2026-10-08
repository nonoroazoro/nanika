import type { SettingsSearchEntry } from "../generated/SettingsSearchEntry";

export interface SettingsSearchGroup
{
    pageId: string;
    title: string;
    entries: SettingsSearchEntry[];
}
