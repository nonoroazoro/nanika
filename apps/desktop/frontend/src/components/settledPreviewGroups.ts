// Remember presentation readiness for this view session, not image buffers.
// Once a group reaches its final artwork, remounting must not show placeholders.
// Active rendering uses the group's reactive completion state; this registry
// only restores settled state when selecting another group.
export const settledGroups = new Set<string>();
