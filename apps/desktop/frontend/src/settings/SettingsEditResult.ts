import type { SettingsValidationError } from "./SettingsValidationError";

/**
 * A commit-ready value or a validation failure that leaves the draft untouched.
 */
export type SettingsEditResult<T> = { error: SettingsValidationError; } | { value: T; };
