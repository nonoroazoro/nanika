export interface SearchResult
{
    extensionId: string;
    entryId: string;
    actionId: string;
    allowDefaultExecution: boolean;
    title: string;
    subtitle: string | null;
    iconUrl: string | null;
    kind: string;
    entryType: "action" | "view";
}
