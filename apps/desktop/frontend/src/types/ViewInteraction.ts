import type { ViewEvent } from "../generated/ViewEvent";

/**
 * Frontend interactions exclude refresh events owned by the host.
 */
export type ViewInteraction = Exclude<ViewEvent, { kind: "invalidated"; }>;
