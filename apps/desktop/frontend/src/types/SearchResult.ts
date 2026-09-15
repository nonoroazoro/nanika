import type { CommandIcon } from "./CommandIcon";

export interface SearchResult
{
    extensionId: string;
    entryId: string;
    actionId: string;
    title: string;
    subtitle: string | null;
    iconUrl: string | null;
    commandIcon: CommandIcon | null;
    kind: string;
}
