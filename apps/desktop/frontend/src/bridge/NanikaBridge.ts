import type {
    ApplicationSnapshot,
    InvokeCandidateRequest,
    PublishQueryRequest,
    RootSearchSnapshot,
    ViewEventReceipt,
    ViewEventRequest
} from "../types";
import type { Action } from "../types/Action";
import type { ContextMenuRequest } from "../types/ContextMenuRequest";

export interface NanikaBridge
{
    readContextMenu(request: ContextMenuRequest): Promise<Action[]>;
    invokeContextMenu(
        request: ContextMenuRequest,
        actionId: string,
        confirmed: boolean
    ): Promise<ViewEventReceipt | null>;
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
