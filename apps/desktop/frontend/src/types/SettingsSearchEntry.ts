import type { SettingsSearchTarget } from "./SettingsSearchTarget";

export interface SettingsSearchEntry
{
    pageId: string;
    target: SettingsSearchTarget;
    title: string;
    pageTitle: string;
}
