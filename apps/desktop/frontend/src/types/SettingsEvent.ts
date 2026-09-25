import type { ExtensionLifecycle } from "./ExtensionLifecycle";
import type { SettingsApplicationUpdate } from "./Settings";

export type SettingsEvent =
    | { deliveryId: number; extensions: ExtensionLifecycle[]; revision: number; type: "lifecycle"; }
    | { deliveryId?: number; type: "application"; update: SettingsApplicationUpdate; }
    | { maximized: boolean; type: "windowState"; }
    | { type: "closed"; }
    | { type: "shortcutPressed"; };
