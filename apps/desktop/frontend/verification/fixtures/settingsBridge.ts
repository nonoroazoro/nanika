import { application } from "./application";
import { failFirstInvocation } from "./invocation";
import { initializationFailure } from "./lifecycle";
import { applicationResources, clipboardResources, clipboardScope, readFixtureResources } from "./localization";
import { clipboard, configuration, currentConfiguration, settings } from "./settings";
import { hostResources } from "../../src/generated/host-resources";
import { ResourceSet } from "../../src/localization/ResourceSet";

import type { settingsBridge as NativeSettingsBridge } from "../../src/bridge/settingsBridge.ts";
import type { ConfigurationSaveError } from "../../src/generated/ConfigurationSaveError";
import type { SettingsSearchEntry } from "../../src/generated/SettingsSearchEntry";

const resources = new ResourceSet(clipboardResources);
const host = new ResourceSet(hostResources);
let revision = 1;
let applicationUpdate: Parameters<typeof NativeSettingsBridge.read>[0] | undefined;
let localize: Parameters<typeof NativeSettingsBridge.read>[6] | undefined;
let navigate: Parameters<typeof NativeSettingsBridge.read>[5] | undefined;
let lifecycle: Parameters<typeof NativeSettingsBridge.read>[4] | undefined;
const variant = new URLSearchParams(location.search).get("state");
if (variant === "language-layout")
{
    settings.general.language = "en";
    settings.localization = { preference: "en", locale: "en", revision: 1 };
}
let rejectResourceRead = variant === "resource-error";
if (variant === "resource-error")
{
    settings.general.language = "zh-Hans";
    settings.localization = { preference: "zh-Hans", locale: "zh-Hans", revision: 1 };
}
let rejectLifecycleChange = false;
let rejectShortcutRecording = variant === "preference-errors";
let rejectDirectoryPick = variant === "preference-errors";
let rejectStartupRead = variant === "host-errors";
let nextSettingsFailure: ConfigurationSaveError | null = null;
let finishLifecycleChange: (() => void) | undefined;

export const settingsBridge: typeof NativeSettingsBridge = {
    readResources: async request =>
    {
        _hostCall("resources");
        if (rejectResourceRead)
        {
            rejectResourceRead = false;
            throw Object.assign(new Error("Fixture resource load is busy"), {
                kind: "host",
                code: "localizationBusy",
                detail: "Original Settings resource <diagnostic>"
            });
        }
        return readFixtureResources(request.scope);
    },
    read: async (_update, _close, _window, _error, onLifecycle, onNavigate, onLocalization) =>
    {
        _hostCall("settingsRead");
        applicationUpdate = _update;
        navigate = onNavigate;
        localize = onLocalization;
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
        if (variant === "progress")
        {
            return structuredClone({
                ...settings,
                navigation: { revision: 1, extensionId: application.id },
                extensions: [application]
            });
        }
        return structuredClone({ ...settings, extensions: variant === "empty" ? [] : settings.extensions });
    },
    ready: async () =>
    {
        if (variant === "host-errors" && _hostCall("ready") === 1)
        {
            throw Object.assign(new Error("Fixture host operation failed"), {
                kind: "host",
                code: "settingsReadyFailed",
                detail: "Original window update <diagnostic>"
            });
        }
        return new URLSearchParams(location.search).get("platform") === "windows";
    },
    search: async query =>
    {
        if (_hostCall("search") === 1 && variant === "host-errors")
        {
            throw Object.assign(new Error("Fixture host operation failed"), {
                kind: "host",
                code: "settingsSearchFailed",
                detail: "Original search <diagnostic>"
            });
        }
        const base = { pageId: clipboard.id, pageTitle: clipboard.name, pageResourceScope: clipboardScope };
        const entries: SettingsSearchEntry[] = [
            { ...base, target: { kind: "page" }, title: clipboard.name, titleResourceScope: clipboardScope },
            { ...base, target: { kind: "enabled" }, title: { message: "settings-enable-extension-title" } },
            ...Object.entries(configuration.properties).map(([key, property]): SettingsSearchEntry => ({
                ...base,
                title: property.title,
                titleResourceScope: clipboardScope,
                target: { kind: "field", key }
            })),
            ...configuration.actions.map((item): SettingsSearchEntry => ({
                ...base,
                title: item.action.title,
                titleResourceScope: clipboardScope,
                target: { kind: "action", key: item.action.id }
            })),
            {
                pageId: "about",
                target: { kind: "page" },
                title: { message: "settings-about" },
                pageTitle: { message: "settings-about" }
            }
        ];
        return entries.filter(entry =>
        {
            let title: string;
            if (typeof entry.title === "string")
            {
                title = entry.title;
            }
            else
            {
                const owner = entry.titleResourceScope ? resources : host;
                title = (["en", "zh-Hans"] as const).map(locale => owner.format(locale, entry.title, "macos", 128))
                    .join(" ");
            }
            const text = entry.pageId === "about" ? title : `Clipboard History 剪贴板历史 ${title}`;
            return text.toLowerCase().includes(query.toLowerCase());
        });
    },
    windowAction: async () =>
    {
        if (variant === "host-errors")
        {
            _hostCall("window");
            throw Object.assign(new Error("Fixture host operation failed"), {
                kind: "host",
                code: "windowCloseFailed",
                detail: "Original close <diagnostic>"
            });
        }
    },
    saveHost: async request =>
    {
        if (variant === "preference-errors" && request.key === "launcherShortcut")
        {
            _hostCall("shortcutWrite");
            if (document.documentElement.dataset.hostShortcutWrite === "1")
            {
                throw Object.assign(new Error("Fixture host transaction failed"), {
                    kind: "HostSettingsFailure",
                    primary: { kind: "host", code: "shortcutReleaseFailed", detail: "Original release <diagnostic>" },
                    rollback: {
                        kind: "host",
                        code: "preferencesRollbackFailed",
                        detail: "Original rollback <diagnostic>"
                    },
                    cleanup: { kind: "host", code: "shortcutCleanupFailed", detail: "Original cleanup <diagnostic>" },
                    confirmation: {
                        kind: "host",
                        code: "preferencesConfirmationFailed",
                        detail: "Original confirmation <diagnostic>"
                    }
                });
            }
        }
        if (request.key === "language" && variant === "language-layout")
        {
            // Preserve the asynchronous native write so intermediate layout is observable.
            await new Promise(resolve =>
            {
                setTimeout(resolve, 120);
            });
        }
        settings.general = { ...settings.general, [request.key]: request.value };
        if (request.key === "language")
        {
            settings.localization = {
                preference: request.value,
                locale: request.value === "zh-Hans" || (request.value === "system" && variant === "language-layout")
                    ? "zh-Hans"
                    : "en",
                revision: settings.localization.revision + 1
            };
            localize?.(settings.sessionId, settings.localization);
        }
        return { values: settings.general, saved: settings.general, effective: settings.general, error: null };
    },
    recordShortcut: async enabled =>
    {
        _hostCall("recording");
        if (enabled && rejectShortcutRecording)
        {
            rejectShortcutRecording = false;
            throw Object.assign(new Error("Fixture recorder unavailable"), {
                kind: "host",
                code: "shortcutRecordingFailed",
                detail: "Original recording <diagnostic>"
            });
        }
        return enabled;
    },
    listenShortcut: () => () =>
    {},
    readStartup: async () =>
    {
        if (variant === "host-errors")
        {
            _hostCall("startupRead");
        }
        if (rejectStartupRead)
        {
            throw Object.assign(new Error("Fixture host operation failed"), {
                kind: "host",
                code: "startupReadFailed",
                detail: "Original startup read <diagnostic>"
            });
        }
        return "disabled";
    },
    setStartup: async enabled =>
    {
        if (variant === "host-errors" && _hostCall("startupWrite") === 1)
        {
            rejectStartupRead = true;
            throw Object.assign(new Error("Fixture host operation failed"), {
                kind: "host",
                code: "startupWriteUnconfirmed",
                detail: "Original startup write <diagnostic>"
            });
        }
        return enabled ? "enabled" : "disabled";
    },
    setEnabled: async (_id, enabled) =>
    {
        document.documentElement.dataset.lifecycleChanges = String(
            Number(document.documentElement.dataset.lifecycleChanges ?? 0) + 1
        );
        if (rejectLifecycleChange)
        {
            rejectLifecycleChange = false;
            await new Promise<void>(resolve =>
            {
                finishLifecycleChange = resolve;
            });
            throw Object.assign(new Error("Fixture lifecycle change failed"), {
                error: { kind: "host", code: "extensionRegistrySaveFailed", detail: "EACCES: /Example/config.json" }
            });
        }
        const extension = clipboard;
        extension.enabled = enabled;
        extension.state = enabled ? "ready" : "disabled";
        extension.lifecycleError = null;
        await lifecycle?.(++revision, [{ ...extension }]);
    },
    invokeAction: async () =>
    {
        if (variant === "invocation-error")
        {
            await failFirstInvocation("error-clear-history");
        }
        document.documentElement.dataset.confirmed = "true";
    },
    save: async request =>
    {
        document.documentElement.dataset.settingsWrites = String(
            Number(document.documentElement.dataset.settingsWrites ?? 0) + 1
        );
        const failure = nextSettingsFailure;
        nextSettingsFailure = null;
        if (failure?.code === "rejected")
        {
            throw Object.assign(new Error("Fixture configuration admission rejected."), failure);
        }
        if (failure)
        {
            return {
                requestId: ++revision,
                extensionId: request.extensionId,
                key: request.key,
                result: { status: "failed", error: failure }
            };
        }
        const current = request.extensionId === application.id ? application.configuration : currentConfiguration;
        if (!current)
        {
            throw new Error("Missing configuration fixture");
        }
        const values = { ...current.values, [request.key]: request.value };
        Object.assign(current, { revision: ++revision, values, saved: values, effective: values });
        return {
            requestId: revision,
            extensionId: request.extensionId,
            key: request.key,
            result: { status: "completed", revision, values, saved: values, effective: values, error: null }
        };
    },
    pickDirectory: async () =>
    {
        _hostCall("directoryPick");
        if (rejectDirectoryPick)
        {
            rejectDirectoryPick = false;
            throw Object.assign(new Error("Fixture folder unavailable"), {
                kind: "host",
                code: "directoryUnavailable",
                detail: "Original folder <diagnostic>"
            });
        }
        return "/Example/Added folder";
    }
};

export function navigateSettings(extensionId?: string): void
{
    navigate?.({ revision: ++revision, extensionId: extensionId ?? null });
}

export async function changeSettingsLanguage(locale: "en" | "zh-Hans"): Promise<void>
{
    await settingsBridge.saveHost({ key: "language", value: locale });
}

export async function publishLifecycleFailure(phase: "restarting" | "restartLimitReached"): Promise<void>
{
    clipboard.pending = phase === "restarting";
    clipboard.state = phase === "restarting" ? "starting" : "failed";
    clipboard.lifecycleError = phase === "restarting"
        ? { phase, failure: initializationFailure }
        : {
            phase,
            original: { error: { kind: "host", code: "extensionExited" }, diagnostic: "first process stderr" },
            failure: initializationFailure
        };
    rejectLifecycleChange = true;
    await lifecycle?.(++revision, [structuredClone(clipboard)]);
}

export function finishLifecycleFailure(): void
{
    finishLifecycleChange?.();
    finishLifecycleChange = undefined;
}

export async function finishSettingsProgress(
    error: "applicationFailure" | ConfigurationSaveError | null = null
): Promise<void>
{
    let failure = error === "applicationFailure" ? null : error;
    if (error === "applicationFailure")
    {
        const resourceSet = new ResourceSet(applicationResources);
        const message = { message: "error-apply-configuration" };
        failure = {
            code: "savedButNotApplied",
            cause: {
                kind: "extension",
                error: {
                    error: {
                        code: "configuration_apply_failed",
                        message,
                        detail: "device permission denied <original>"
                    },
                    messages: {
                        en: resourceSet.format("en", message, "macos", 2048, false),
                        "zh-Hans": resourceSet.format("zh-Hans", message, "macos", 2048, false)
                    }
                }
            }
        };
    }
    const update = application.application;
    const current = application.configuration;
    if (!update || !current)
    {
        throw new Error("Missing application progress fixture");
    }
    await applicationUpdate?.({
        ...update,
        result: {
            status: "completed",
            revision: 2,
            values: current.values,
            saved: current.saved,
            effective: failure ? null : current.values,
            error: failure
        }
    });
    document.documentElement.dataset.progressCompleted = "true";
}

export async function publishForeignProgress(): Promise<void>
{
    const update = application.application;
    if (!update)
    {
        throw new Error("Missing application progress fixture");
    }
    await applicationUpdate?.({
        ...update,
        resourceScope: clipboardScope,
        result: {
            status: "running",
            progress: { label: { message: "progress-scanning-sources" }, completed: 2, total: 3 }
        }
    });
}

export function failNextSettingsWrite(code: "rejected" | "unconfirmed"): void
{
    nextSettingsFailure = {
        code,
        cause: {
            kind: "host",
            code: code === "rejected" ? "operationPending" : "configurationUnconfirmed",
            detail: "fixture original diagnostic <retained>"
        }
    };
}

export function setStartupReadFailure(reject: boolean): void
{
    rejectStartupRead = reject;
}

function _hostCall(operation: string): number
{
    const key = `host${operation[0]?.toUpperCase()}${operation.slice(1)}`;
    const count = Number(document.documentElement.dataset[key] ?? 0) + 1;
    document.documentElement.dataset[key] = String(count);
    return count;
}
