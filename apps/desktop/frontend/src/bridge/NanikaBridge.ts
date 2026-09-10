import type { ApplicationSnapshot, InvokeCandidateRequest, PublishQueryRequest, RootSearchSnapshot } from "../types";

export interface NanikaBridge
{
    openSession(
        listener: (snapshot: RootSearchSnapshot) => void,
        onError: (error: unknown) => void
    ): Promise<ApplicationSnapshot>;
    closeSession(sessionId: number): Promise<void>;
    publishQuery(request: PublishQueryRequest): Promise<void>;
    invokeCandidate(request: InvokeCandidateRequest): Promise<boolean>;
    dismissLauncher(): Promise<void>;
}
