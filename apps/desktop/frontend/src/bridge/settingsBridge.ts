import { Channel, invoke } from "@tauri-apps/api/core";

import type { ExtensionLifecycle } from "../generated/ExtensionLifecycle";
import type { HostSettingsChange } from "../generated/HostSettingsChange";
import type { LauncherPreferences } from "../generated/LauncherPreferences";
import type { SaveSettingsRequest } from "../generated/SaveSettingsRequest";
import type { SettingsApplicationUpdate } from "../generated/SettingsApplicationUpdate";
import type { SettingsEvent } from "../generated/SettingsEvent";
import type { SettingsSearchEntry } from "../generated/SettingsSearchEntry";
import type { SettingsSnapshot } from "../generated/SettingsSnapshot";
import type { SettingsWindowAction } from "../generated/SettingsWindowAction";
import type { SettingsWriteResult } from "../generated/SettingsWriteResult";
import type { StartupStatus } from "../generated/StartupStatus";

// A retained Settings WebView owns one channel, including across native closes.
let updates: Channel<SettingsEvent> | undefined;
let onShortcut: (() => void) | undefined;

export const settingsBridge = {
    read: async (
        onUpdate: (update: SettingsApplicationUpdate) => void,
        onClose: () => void,
        onWindowState: (maximized: boolean) => void,
        onError: (error: unknown) => void,
        onLifecycle: (revision: number, extensions: ExtensionLifecycle[]) => void
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
            else if (event.type === "lifecycle")
            {
                onLifecycle(event.revision, event.extensions);
            }
            else
            {
                onUpdate(event.update);
            }
            if ("deliveryId" in event && event.deliveryId !== undefined)
            {
                void invoke("acknowledge_settings_delivery", { deliveryId: event.deliveryId }).catch(onError);
            }
        };
        // Rust registers before loading configuration; retain the channel even if loading fails.
        return await invoke("read_settings", { updates: subscribe ? updates : null });
    },
    search: async (query: string): Promise<SettingsSearchEntry[]> =>
    {
        const results = new Channel<SettingsSearchEntry[]>();
        const delivery = new Promise<SettingsSearchEntry[]>(resolve =>
        {
            results.onmessage = resolve;
        });
        // The invoke acknowledges transport work; only the window-bound Channel supplies results.
        await invoke<null>("search_settings", { query, results });
        return await delivery;
    },
    ready: async (): Promise<boolean> => invoke("settings_ready"),
    windowAction: async (action: SettingsWindowAction): Promise<void> => invoke("settings_window_action", { action }),
    saveHost: async (request: HostSettingsChange): Promise<SettingsWriteResult<LauncherPreferences>> =>
        invoke("save_host_settings", { request }),
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
    setEnabled: async (extensionId: string, enabled: boolean): Promise<void> =>
        invoke("set_extension_enabled", { extensionId, enabled }),
    save: async (request: SaveSettingsRequest): Promise<SettingsApplicationUpdate> =>
        invoke("save_settings", { request }),
    pickDirectory: async (extensionId: string, key: string): Promise<string | null> =>
        invoke("pick_settings_directory", { extensionId, key })
};
