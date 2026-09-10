import type { SearchPhase, SearchResult } from "./index";

export interface RootSearchSnapshot
{
    sessionId: number;
    requestId: number;
    revision: number;
    query: string;
    results: SearchResult[];
    phase: SearchPhase;
    error: string | null;
    warnings: string[];
}
