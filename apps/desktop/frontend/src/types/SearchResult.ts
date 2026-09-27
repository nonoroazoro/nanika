import type { CandidateSubtitle } from "./CandidateSubtitle";
import type { ResultIcon } from "./ResultIcon";

export interface SearchResult
{
    extensionId: string;
    entryId: string;
    actionId: string;
    allowDefaultExecution: boolean;
    confirmationTitle: string | null;
    title: string;
    subtitle: CandidateSubtitle | null;
    icon: ResultIcon | null;
    kind: string;
    entryType: "action" | "view";
}
