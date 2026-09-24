import { Channel, invoke } from "@tauri-apps/api/core";

import type {
    ConfigurationValue,
    HostPreferences,
    SettingsApplicationUpdate,
    SettingsSnapshot,
    StartupStatus
} from "../types/Settings";
import type { SettingsEvent } from "../types/SettingsEvent";
import type { SettingsWindowAction } from "../types/SettingsWindowAction";

// A retained Settings WebView owns one channel, including across native closes.
let updates: Channel<SettingsEvent> | undefined;
let onShortcut: (() => void) | undefined;

export const settingsBridge = {
    read: async (
        onUpdate: (update: SettingsApplicationUpdate) => void,
        onClose: () => void,
        onWindowState: (maximized: boolean) => void
    ): Promise<SettingsSnapshot> =>
    {
        const subscribe = updates === undefined;
        updates = updates ?? new Channel<SettingsEvent>();
        updates.onmessage = event =>
        {
            if (event.type === "closed")
            {
                onClose();
            }
            else if (event.type === "windowState")
            {
                onWindowState(event.maximized);
            }
            else if (event.type === "shortcutPressed")
            {
                onShortcut?.();
            }
            else
            {
                onUpdate(event.update);
            }
        };
        // Registration precedes configuration loading in Rust, so a failed read
        // still retains this channel for window events and a later retry.
        return await invoke("read_settings", { updates: subscribe ? updates : null });
    },
    ready: async (): Promise<boolean> => invoke("settings_ready"),
    windowAction: async (action: SettingsWindowAction): Promise<void> => invoke("settings_window_action", { action }),
    saveHost: async (preferences: HostPreferences): Promise<HostPreferences> =>
        invoke("save_host_settings", { preferences }),
    recordShortcut: async (recording: boolean): Promise<boolean> => invoke("set_shortcut_recording", { recording }),
    listenShortcut: (handler: () => void): () => void =>
    {
        onShortcut = handler;
        return () =>
        {
            if (onShortcut === handler)
            {
                onShortcut = undefined;
            }
        };
    },
    readStartup: async (): Promise<StartupStatus> => invoke("read_startup"),
    setStartup: async (enabled: boolean): Promise<StartupStatus> => invoke("set_startup", { enabled }),
    save: async (extensionId: string, values: Record<string, ConfigurationValue>): Promise<SettingsApplicationUpdate> =>
        invoke("save_settings", { request: { extensionId, values } }),
    pickDirectory: async (extensionId: string, key: string): Promise<string | null> =>
        invoke("pick_settings_directory", { extensionId, key })
};
