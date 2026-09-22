// Retain settled readiness across group remounts without retaining image buffers.
export const settledGroups = new Set<string>();
