import type { CandidateSubtitle } from "./CandidateSubtitle";

export interface SearchResult
{
    extensionId: string;
    entryId: string;
    actionId: string;
    allowDefaultExecution: boolean;
    title: string;
    subtitle: CandidateSubtitle | null;
    iconUrl: string | null;
    kind: string;
    entryType: "action" | "view";
}
