import type { SettingsApplicationUpdate } from "./Settings";

export type SettingsEvent =
    | { maximized: boolean; type: "windowState"; }
    | { type: "application"; update: SettingsApplicationUpdate; }
    | { type: "closed"; }
    | { type: "shortcutPressed"; };
