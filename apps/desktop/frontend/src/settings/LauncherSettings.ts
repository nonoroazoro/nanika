import type { HostSettingsChange } from "../generated/HostSettingsChange";
import type { LauncherPreferences } from "../generated/LauncherPreferences";

/**
 * Editable preferences exclude storage metadata such as formatVersion.
 */
export type LauncherSettings = Pick<LauncherPreferences, HostSettingsChange["key"]>;
