import type { ViewInteraction } from "./ViewInteraction";
import type { ViewEventRequest } from "../generated/ViewEventRequest";

/**
 * Keeps wire fields while restricting events to frontend-owned interactions.
 */
export type ViewInteractionRequest = {
    operation:
        | ({ event: ViewInteraction; } & Extract<ViewEventRequest["operation"], { kind: "event"; }>)
        | Exclude<ViewEventRequest["operation"], { kind: "event"; }>;
} & Omit<ViewEventRequest, "operation">;
