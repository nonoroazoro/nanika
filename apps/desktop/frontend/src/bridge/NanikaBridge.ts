import type {
    ApplicationSnapshot,
    InvokeCandidateRequest,
    PublishQueryRequest,
    RootSearchSnapshot,
    ViewEventReceipt,
    ViewEventRequest
} from "../types";

export interface NanikaBridge
{
    openSession(
        listener: (snapshot: RootSearchSnapshot) => void,
        onError: (error: unknown) => void
    ): Promise<ApplicationSnapshot>;
    closeSession(sessionId: number): Promise<void>;
    publishQuery(request: PublishQueryRequest): Promise<void>;
    refreshSearch(sessionId: number): Promise<void>;
    invokeCandidate(request: InvokeCandidateRequest): Promise<void>;
    viewEvent(request: ViewEventRequest): Promise<ViewEventReceipt>;
    dismissLauncher(): Promise<void>;
    openSettings(): Promise<void>;
}
