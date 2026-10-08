import type { ReadResultsRequest as WireRequest } from "../generated/ReadResultsRequest";

/**
 * Callers select a range; the bridge assigns its delivery sequence.
 */
export type ReadResultsRequest = Omit<WireRequest, "rangeId">;
