export interface InvokeCandidateRequest
{
    sessionId: number;
    requestId: number;
    resultRevision: number;
    extensionId: string;
    entryId: string;
    actionId: string;
}
