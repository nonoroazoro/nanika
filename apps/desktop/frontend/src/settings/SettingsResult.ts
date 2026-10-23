import type { SettingsError } from "./SettingsError";
import type { SettingsWriteResult } from "../generated/SettingsWriteResult";

/**
 * Shared settings state accepts host writes and typed extension transaction outcomes.
 */
export type SettingsResult<T> = { error: SettingsError | null; } & Omit<SettingsWriteResult<T>, "error">;
