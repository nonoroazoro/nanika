import type { ContributionIcon } from "./ContributionIcon";

export interface SearchResult
{
    extensionId: string;
    entryId: string;
    actionId: string;
    title: string;
    subtitle: string | null;
    iconUrl: string | null;
    contributionIcon: ContributionIcon | null;
    kind: string;
    entryType: "action" | "view";
}
