import type { NavigationSnapshot as NavigationUpdate } from "../generated/NavigationSnapshot";

/**
 * Complete navigation after the bridge applies an incremental update.
 */
export type NavigationSnapshot = Required<NavigationUpdate>;
