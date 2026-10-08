import type { Action } from "../generated/Action";
import type { ApplicationSnapshot } from "../generated/ApplicationSnapshot";
import type { ContextMenuRequest } from "../generated/ContextMenuRequest";
import type { InvokeCandidateRequest } from "../generated/InvokeCandidateRequest";
import type { PublishQueryRequest } from "../generated/PublishQueryRequest";
import type { ViewEventReceipt } from "../generated/ViewEventReceipt";
import type { ReadResultsRequest } from "../types/ReadResultsRequest";
import type { RootSearchSnapshot } from "../types/RootSearchSnapshot";
import type { ViewInteractionRequest } from "../types/ViewInteractionRequest";

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
    readResults(request: ReadResultsRequest): Promise<void>;
    publishQuery(request: PublishQueryRequest): Promise<void>;
    invokeCandidate(request: InvokeCandidateRequest): Promise<void>;
    viewEvent(request: ViewInteractionRequest): Promise<ViewEventReceipt>;
    dismissLauncher(): Promise<void>;
    openSettings(): Promise<void>;
}
