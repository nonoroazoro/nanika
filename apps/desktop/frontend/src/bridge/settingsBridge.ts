import { Channel, invoke } from "@tauri-apps/api/core";

import type {
    ConfigurationValue,
    HostPreferences,
    SettingsApplicationUpdate,
    SettingsSnapshot,
    StartupStatus
} from "../types/Settings";

type SettingsEvent = { type: "application"; update: SettingsApplicationUpdate; } | { type: "closed"; } | {
    type: "shortcutPressed";
};
// A retained Settings WebView owns one channel, including across native closes.
let updates: Channel<SettingsEvent> | undefined;
let onShortcut: (() => void) | undefined;

export const settingsBridge = {
    read: async (
        onUpdate: (update: SettingsApplicationUpdate) => void,
        onClose: () => void
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
            else if (event.type === "shortcutPressed")
            {
                onShortcut?.();
            }
            else
            {
                onUpdate(event.update);
            }
        };
        try
        {
            // Rust channels have their own sequence and end-of-stream marker.
            // Re-deserializing this callback ID would reset that sequence.
            return await invoke("read_settings", { updates: subscribe ? updates : null });
        }
        catch (cause)
        {
            if (subscribe)
            {
                updates = undefined;
            }
            throw cause;
        }
    },
    ready: async (): Promise<void> => invoke("settings_ready"),
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
