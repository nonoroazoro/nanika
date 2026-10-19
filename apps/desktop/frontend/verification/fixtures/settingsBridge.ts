import { clipboard, configuration, currentConfiguration, settings } from "./settings";

import type { settingsBridge as NativeSettingsBridge } from "../../src/bridge/settingsBridge.ts";
import type { SettingsSearchEntry } from "../../src/generated/SettingsSearchEntry";

let revision = 1;
let navigate: Parameters<typeof NativeSettingsBridge.read>[5] | undefined;
let lifecycle: Parameters<typeof NativeSettingsBridge.read>[4] | undefined;
const variant = new URLSearchParams(location.search).get("state");

export const settingsBridge: typeof NativeSettingsBridge = {
    read: async (_update, _close, _window, _error, onLifecycle, onNavigate) =>
    {
        navigate = onNavigate;
        lifecycle = onLifecycle;
        if (variant === "loading")
        {
            await new Promise(() =>
            {});
        }
        if (variant === "failure")
        {
            throw new Error("Fixture configuration unavailable");
        }
        return structuredClone({ ...settings, extensions: variant === "empty" ? [] : settings.extensions });
    },
    ready: async () => new URLSearchParams(location.search).get("platform") === "windows",
    search: async query =>
    {
        const base = { pageId: "com.nanika.clipboard", pageTitle: "Clipboard History" };
        const entries: SettingsSearchEntry[] = [
            { ...base, target: { kind: "page" }, title: "Clipboard History" },
            { ...base, target: { kind: "enabled" }, title: "Enable extension" },
            ...Object.entries(configuration.properties).map(([key, property]): SettingsSearchEntry => ({
                ...base,
                title: property.title,
                target: { kind: "field", key }
            })),
            ...configuration.actions.map((item): SettingsSearchEntry => ({
                ...base,
                title: item.action.title,
                target: { kind: "action", key: item.action.id }
            }))
        ];
        return entries.filter(entry => `${entry.pageTitle} ${entry.title}`.toLowerCase().includes(query.toLowerCase()));
    },
    windowAction: async () =>
    {},
    saveHost: async request =>
    {
        settings.general = { ...settings.general, [request.key]: request.value };
        return { values: settings.general, saved: settings.general, effective: settings.general, error: null };
    },
    recordShortcut: async () => true,
    listenShortcut: () => () =>
    {},
    readStartup: async () => "disabled",
    setStartup: async enabled => (enabled ? "enabled" : "disabled"),
    setEnabled: async (_id, enabled) =>
    {
        const extension = clipboard;
        extension.enabled = enabled;
        extension.state = enabled ? "ready" : "disabled";
        lifecycle?.(++revision, [{ ...extension }]);
    },
    invokeAction: async () =>
    {
        document.documentElement.dataset.confirmed = "true";
    },
    save: async request =>
    {
        const current = currentConfiguration;
        const values = { ...current.values, [request.key]: request.value };
        Object.assign(current, { revision: ++revision, values, saved: values, effective: values });
        return {
            requestId: revision,
            extensionId: request.extensionId,
            key: request.key,
            result: { status: "completed", revision, values, saved: values, effective: values, error: null }
        };
    },
    pickDirectory: async () => "/Example/Added folder"
};

export function navigateSettings(extensionId?: string): void
{
    navigate?.({ revision: ++revision, extensionId: extensionId ?? null });
}
