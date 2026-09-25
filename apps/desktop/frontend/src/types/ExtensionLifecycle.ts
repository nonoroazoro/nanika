import type { ExtensionSettings } from "./Settings";

export type ExtensionLifecycle = Omit<ExtensionSettings, "application">;
