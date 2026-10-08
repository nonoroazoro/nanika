import type { NavigationSnapshot } from "./NavigationSnapshot";
import type { RootSearchSnapshot as SearchUpdate } from "../generated/RootSearchSnapshot";

/**
 * Complete search state after the bridge retains omitted results and navigation.
 */
export type RootSearchSnapshot = { navigation: NavigationSnapshot; } & Omit<Required<SearchUpdate>, "navigation">;
