import type { SettingsValidationError } from "./SettingsValidationError";
import type { ConfigurationSaveError } from "../generated/ConfigurationSaveError";
import type { HostSettingsFailure } from "../generated/HostSettingsFailure";
import type { LifecycleFailure } from "../generated/LifecycleFailure";
import type { OperationError } from "../generated/OperationError";

/**
 * Typed configuration facts, operation failures and validation constraints.
 */
export type SettingsError =
    | ConfigurationSaveError
    | HostSettingsFailure
    | LifecycleFailure
    | OperationError
    | SettingsValidationError;
