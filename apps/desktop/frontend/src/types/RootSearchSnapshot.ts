import type { NavigationSnapshot, SearchPhase, SearchResult } from "./index";

export interface RootSearchSnapshot
{
    navigation: NavigationSnapshot;
    sessionId: number;
    requestId: number;
    revision: number;
    query: string;
    results: SearchResult[];
    resultRevision: number;
    resultOffset: number;
    totalResults: number;
    phase: SearchPhase;
    error: string | null;
    warnings: string[];
}
