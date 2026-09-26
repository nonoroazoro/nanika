export interface ReadResultsRequest
{
    sessionId: number;
    requestId: number;
    resultRevision: number;
    offset: number;
    count: number;
}
