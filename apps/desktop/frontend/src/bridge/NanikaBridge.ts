import type { Action } from "../generated/Action";
import type { ApplicationSnapshot } from "../generated/ApplicationSnapshot";
import type { ConfigurationMenu } from "../generated/ConfigurationMenu";
import type { ConfigurationMenuOperation } from "../generated/ConfigurationMenuOperation";
import type { ConfigurationMenuRequest } from "../generated/ConfigurationMenuRequest";
import type { ContextMenuRequest } from "../generated/ContextMenuRequest";
import type { InvokeCandidateRequest } from "../generated/InvokeCandidateRequest";
import type { LifecycleFailure } from "../generated/LifecycleFailure";
import type { LocalizationResourceRequest } from "../generated/LocalizationResourceRequest";
import type { PreparedResources } from "../generated/PreparedResources";
import type { PublishQueryRequest } from "../generated/PublishQueryRequest";
import type { ReadTextRequest } from "../generated/ReadTextRequest";
import type { TextChunk } from "../generated/TextChunk";
import type { ViewEventReceipt } from "../generated/ViewEventReceipt";
import type { ReadResultsRequest } from "../types/ReadResultsRequest";
import type { RootSearchSnapshot } from "../types/RootSearchSnapshot";
import type { ViewInteractionRequest } from "../types/ViewInteractionRequest";

export interface NanikaBridge
{
    readResources(request: Extract<LocalizationResourceRequest, { surface: "launcher"; }>): Promise<PreparedResources>;
    readConfigurationMenu(request: ConfigurationMenuRequest): Promise<ConfigurationMenu>;
    invokeConfigurationMenu(request: ConfigurationMenuRequest, operation: ConfigurationMenuOperation): Promise<void>;
    readText(request: ReadTextRequest): Promise<TextChunk>;
    readContextMenu(request: ContextMenuRequest): Promise<Action[]>;
    invokeContextMenu(
        request: ContextMenuRequest,
        actionId: string,
        confirmed: boolean
    ): Promise<ViewEventReceipt | null>;
    openSession(
        listener: (snapshot: RootSearchSnapshot) => Promise<void> | void,
        onError: (error: unknown) => void
    ): Promise<ApplicationSnapshot>;
    closeSession(sessionId: number): Promise<void>;
    readStartupDiagnostic(sessionId: number): Promise<LifecycleFailure>;
    readResults(request: ReadResultsRequest): Promise<void>;
    publishQuery(request: PublishQueryRequest): Promise<void>;
    invokeCandidate(request: InvokeCandidateRequest): Promise<void>;
    viewEvent(request: ViewInteractionRequest): Promise<ViewEventReceipt>;
    dismissLauncher(): Promise<void>;
    openSettings(extensionId?: string): Promise<void>;
}
