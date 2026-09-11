import type { ViewEvent } from "./index";
export type ViewOperation =
    | { event: ViewEvent; kind: "event"; }
    | { kind: "back"; };
export interface ViewEventRequest
{
    sessionId: number;
    routeId: number;
    revision: number;
    operation: ViewOperation;
}
