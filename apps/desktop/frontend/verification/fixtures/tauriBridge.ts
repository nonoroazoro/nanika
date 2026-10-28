import { failFirstInvocation } from "./invocation";
import {
    applicationResources,
    applicationScope,
    clipboardResources,
    clipboardScope,
    readFixtureResources,
    systemResources,
    systemScope
} from "./localization";
import { clipboard, configuration } from "./settings";
import { navigateSettings } from "./settingsBridge";
import { localizedFiles, localizedList, longLabels, textReadFailure } from "./view";
import applicationIcon from "../../../../extensions/built-in/application/assets/icon.png?url";
import { operationFailure } from "../../src/localization/operation-errors";
import { ResourceSet } from "../../src/localization/ResourceSet";

import type { NanikaBridge } from "../../src/bridge/NanikaBridge";
import type { ConfigurationSaveError } from "../../src/generated/ConfigurationSaveError";
import type { OperationError } from "../../src/generated/OperationError";
import type { SearchResult } from "../../src/generated/SearchResult";
import type { View } from "../../src/generated/View";
import type { RootSearchSnapshot } from "../../src/types/RootSearchSnapshot";

const clearAction = configuration.actions[0]?.action;
if (!clearAction)
{
    throw new Error("Clear action fixture is missing");
}
const snippet = "const message = 'Hello, Nanika';\n";
let syntaxHighlighting = true;
let configurationFailure: ConfigurationSaveError | undefined;
const params = new URLSearchParams(location.search);
const variant = params.get("state");
let textReadAvailable = false;
let viewUpdateFailed = false;
let viewCloseFailed = false;
let finishViewFailure: (() => void) | undefined;
const localizedViews: Record<string, View> = {
    "localized-list": localizedList,
    "long-labels": longLabels,
    "localized-files": localizedFiles,
    "text-read-error": textReadFailure,
    "view-error": localizedList
};
const results: SearchResult[] = ([
    {
        title: "Clipboard History",
        extensionId: "com.nanika.clipboard",
        subtitle: { kind: "label", text: "Clipboard" },
        icon: { kind: "image", url: clipboard.iconUrl },
        entryType: "view"
    },
    {
        title: { message: "action-trash-empty-title" },
        extensionId: "com.nanika.system",
        subtitle: { kind: "label", text: { message: "extension-name" } },
        icon: { kind: "symbol", name: "trash" },
        entryType: "action"
    },
    {
        title: { message: "action-trash-open-title" },
        extensionId: "com.nanika.system",
        subtitle: { kind: "label", text: { message: "extension-name" } },
        icon: { kind: "symbol", name: "folder-open" },
        entryType: "action"
    },
    {
        title: variant === "isolated-title"
            ? "shortcut\u206842\u2069"
            : "A long application name for overflow and truncation verification",
        extensionId: "com.nanika.application",
        subtitle: { kind: "label", text: "Application" },
        icon: { kind: "image", url: applicationIcon },
        entryType: "action"
    }
] satisfies Array<Pick<SearchResult, "entryType" | "extensionId" | "icon" | "subtitle" | "title">>).map((
    entry,
    index
) => ({
    ...entry,
    instanceId: 1,
    resourceScope: entry.extensionId === systemScope.extensionId ? systemScope : null,
    entryId: String(index),
    actionId: "open",
    actions: [{
        id: "open",
        title: index === 1 ? { message: "action-trash-empty-title" } : entry.title,
        style: index === 1 ? "destructive" : "primary",
        enabled: true,
        allow_default_execution: index !== 1,
        group: null,
        ...(index === 1
            ? {
                confirmation_title: { message: "action-trash-empty-confirmation-title" },
                confirmation_description: { message: "action-trash-empty-confirmation-description" }
            }
            : {})
    }],
    kind: "Extension"
}));
if (variant === "shortcuts")
{
    const result = results[3];
    if (!result)
    {
        throw new Error("Shortcut result fixture is missing");
    }
    result.extensionId = "external.shortcuts";
    result.actions.push(
        {
            id: "custom.location",
            title: "Show location",
            style: "secondary",
            enabled: true,
            allow_default_execution: false,
            group: null,
            shortcut: { key: { kind: "virtual", value: "Enter" }, modifiers: ["primary"] }
        },
        {
            id: "custom.inspect",
            title: "Inspect",
            style: "secondary",
            enabled: true,
            allow_default_execution: false,
            group: null,
            shortcut: { key: { kind: "physical", value: "KeyK" }, modifiers: ["primary", "shift"] }
        },
        {
            id: "custom.disabled",
            title: "Unavailable",
            style: "secondary",
            enabled: false,
            allow_default_execution: false,
            group: null,
            shortcut: { key: { kind: "virtual", value: "9" }, modifiers: ["primary"] }
        },
        {
            id: "custom.confirm",
            title: "Confirm action",
            style: "destructive",
            enabled: true,
            allow_default_execution: false,
            confirmation_title: "Proceed with this action?",
            group: null,
            shortcut: { key: { kind: "virtual", value: "Enter" }, modifiers: ["primary", "shift"] }
        }
    );
}
if (variant === "scroll-layout")
{
    const source = results.at(-1);
    if (!source)
    {
        throw new Error("Application fixture is missing");
    }
    results.push(...Array.from({ length: 30 }, (_, index) => ({
        ...source,
        entryId: `scroll-${index}`,
        title: `Application ${index}`
    })));
}
const resources = new ResourceSet(systemResources);
const searchTitles = new Map(
    results.map(result => [
        result.entryId,
        (["en", "zh-Hans"] as const).map(locale =>
        {
            return resources.format(
                locale,
                result.title,
                params.get("platform") === "windows" ? "windows" : "macos",
                128
            ).replace(/[\u2066-\u2069]/gu, "").toLowerCase();
        })
    ])
);
let listener: Parameters<NanikaBridge["openSession"]>[0] | undefined;
let publishedResults: SearchResult[] = [];
let snapshot: RootSearchSnapshot = {
    sessionId: 1,
    localization: { preference: "system", locale: "en", revision: 1 },
    requestId: 0,
    revision: 1,
    resultRevision: 1,
    resultOffset: 0,
    totalResults: results.length,
    query: "",
    phase: "ready",
    error: null,
    results,
    pendingExtensions: [],
    warnings: [],
    textInvalidations: [],
    navigation: { revision: 1, current: null, busy: false, error: null, dismissCount: 0 }
};

export const tauriBridge: NanikaBridge = {
    readResources: async request => readFixtureResources(request.scope),
    readStartupDiagnostic: async sessionId =>
    {
        if (
            sessionId !== snapshot.sessionId
            || (variant !== "startup-failure" && variant !== "startup-diagnostic-error")
        )
        {
            throw new Error("No startup diagnostic");
        }
        document.documentElement.dataset.startupDiagnosticReads = String(
            Number(document.documentElement.dataset.startupDiagnosticReads ?? 0) + 1
        );
        if (variant === "startup-diagnostic-error" && document.documentElement.dataset.startupDiagnosticReads === "1")
        {
            throw Object.assign(new Error("Startup diagnostic transport failed"), {
                kind: "host",
                code: "startupDiagnosticUnavailable"
            });
        }
        return {
            error: { kind: "host", code: "hostStorageUnavailable" },
            diagnostic: `SQLITE_CANTOPEN: /Example/私密/nanika.db\n${"Original diagnostic detail\n".repeat(80)}`
        };
    },
    openSession: async callback =>
    {
        listener = callback;
        if (variant === "failure")
        {
            throw new Error("Fixture search unavailable");
        }
        if (variant === "startup-failure" || variant === "startup-diagnostic-error")
        {
            snapshot = {
                ...snapshot,
                results: [],
                totalResults: 0,
                phase: "error",
                error: { kind: "host", code: "hostStorageUnavailable" }
            };
        }
        if (variant === "loading" || variant === "empty")
        {
            snapshot = {
                ...snapshot,
                results: [],
                totalResults: 0,
                phase: variant === "loading" ? "searching" : "ready"
            };
        }
        _publish();
        return {
            sessionId: 1,
            localization: { preference: "system", locale: "en", revision: 1 },
            version: "0.1.0",
            platform: params.get("platform") === "windows" ? "windows" : "macos",
            maxQueryChars: 1024,
            resourceOrigin: location.origin
        };
    },
    closeSession: async () =>
    {
        listener = undefined;
    },
    publishQuery: async request =>
    {
        document.documentElement.dataset.queryCount = String(
            Number(document.documentElement.dataset.queryCount ?? 0) + 1
        );
        document.documentElement.dataset.maximumQueryLength = String(
            Math.max(Number(document.documentElement.dataset.maximumQueryLength ?? 0), Array.from(request.query).length)
        );
        if (variant === "query-error" && request.query === "app")
        {
            throw Object.assign(new Error("Fixture host operation failed"), {
                kind: "host",
                code: "searchSubmissionFailed",
                detail: "Original query <diagnostic>"
            });
        }
        const filtered = results.filter(item =>
        {
            return searchTitles.get(item.entryId)?.some(title => title.includes(request.query.toLowerCase()));
        });
        snapshot = {
            ...snapshot,
            query: request.query,
            requestId: request.requestId,
            phase: "searching",
            results: [],
            totalResults: 0
        };
        _publish();
        await new Promise(resolve =>
        {
            requestAnimationFrame(resolve);
        });
        snapshot = {
            ...snapshot,
            query: request.query,
            requestId: request.requestId,
            results: filtered,
            totalResults: filtered.length,
            phase: "ready",
            resultRevision: snapshot.resultRevision + 1
        };
        _publish();
    },
    readResults: async () =>
    {},
    invokeCandidate: async request =>
    {
        if (variant === "shortcuts" && request.entryId === "3")
        {
            document.documentElement.dataset.shortcutInvocations = String(
                Number(document.documentElement.dataset.shortcutInvocations ?? 0) + 1
            );
            document.documentElement.dataset.shortcutRequest = JSON.stringify(request);
            while (document.documentElement.dataset.shortcutPaused === "true")
            {
                await new Promise(resolve =>
                {
                    requestAnimationFrame(resolve);
                });
            }
            if (document.documentElement.dataset.shortcutFailure === "true")
            {
                throw Object.assign(new Error("Fixture action failed"), { kind: "host", code: "actionUnavailable" });
            }
            return;
        }
        if (variant === "invocation-error" || variant === "host-service-error")
        {
            snapshot.navigation = {
                ...snapshot.navigation,
                busy: true,
                error: null,
                revision: snapshot.navigation.revision + 1
            };
            _publish();
            try
            {
                await failFirstInvocation(
                    variant === "host-service-error" ? "error-host-service" : "error-open-history"
                );
            }
            catch (error)
            {
                snapshot.navigation = {
                    ...snapshot.navigation,
                    busy: false,
                    error: operationFailure(error, "invocationFailed"),
                    revision: snapshot.navigation.revision + 1
                };
                _publish();
                throw error;
            }
            snapshot.navigation = { ...snapshot.navigation, busy: false };
        }
        if (request.entryId !== "0")
        {
            const dialog = document.querySelector("[role=alertdialog]");
            document.documentElement.dataset.confirmationAtInvoke = JSON.stringify({
                confirmed: request.invocation === "confirmed",
                opacity: dialog ? Number(getComputedStyle(dialog).opacity) : 0
            });
            document.documentElement.dataset.invocationCount = String(
                Number(document.documentElement.dataset.invocationCount ?? 0) + 1
            );
            document.documentElement.dataset.confirmed = "true";
            snapshot.navigation = {
                ...snapshot.navigation,
                revision: snapshot.navigation.revision + 1,
                dismissCount: snapshot.navigation.dismissCount + 1
            };
            _publish();
            return;
        }
        snapshot.navigation = {
            ...snapshot.navigation,
            revision: snapshot.navigation.revision + 1,
            current: {
                routeId: 1,
                extensionId: "com.nanika.clipboard",
                extensionName: { message: "extension-name" },
                resourceScope: clipboardScope,
                extensionIconUrl: clipboard.iconUrl,
                instanceId: 1,
                generation: 1,
                viewId: "clipboard.history",
                revision: 1,
                view: localizedViews[variant ?? ""] ?? {
                    kind: "detail",
                    detail: {
                        title: "Code preview",
                        content: {
                            kind: "text",
                            value: snippet,
                            syntax_highlighting: true,
                            document: {
                                id: "snippet",
                                revision: "1",
                                utf16Length: snippet.length,
                                lineCount: 2,
                                maxLineLength: snippet.trimEnd().length,
                                totalChunks: 1
                            }
                        },
                        metadata: [],
                        actions: []
                    }
                }
            }
        };
        _publish();
    },
    viewEvent: async request =>
    {
        document.documentElement.dataset.viewEventCount = String(
            Number(document.documentElement.dataset.viewEventCount ?? 0) + 1
        );
        const failUpdate = variant === "view-error" && !viewUpdateFailed
            && request.operation.kind === "event" && request.operation.event.kind === "filterChanged";
        const failClose = variant === "view-error" && !viewCloseFailed && request.operation.kind === "back";
        if (failUpdate || failClose)
        {
            viewUpdateFailed = viewUpdateFailed || failUpdate;
            viewCloseFailed = viewCloseFailed || failClose;
            snapshot.navigation = {
                ...snapshot.navigation,
                busy: true,
                error: null,
                revision: snapshot.navigation.revision + 1
            };
            _publish();
            await new Promise<void>(resolve =>
            {
                finishViewFailure = resolve;
            });
            const message = { message: failClose ? "error-close-history" : "error-update-history" };
            const errorResources = new ResourceSet(clipboardResources);
            const failure: OperationError = {
                kind: "extension",
                error: {
                    error: {
                        code: failClose ? "view_close_failed" : "view_update_failed",
                        message,
                        detail: `EACCES: /Example/历史.db${
                            failClose ? "\nBounded original diagnostic".repeat(300) : ""
                        }`
                    },
                    messages: {
                        en: errorResources.format("en", message, "macos", 2048, false),
                        "zh-Hans": errorResources.format("zh-Hans", message, "macos", 2048, false)
                    }
                }
            };
            const error: OperationError = failClose
                ? { kind: "cleanup", primary: { kind: "host", code: "navigationDepthReached" }, cleanup: failure }
                : failure;
            snapshot.navigation = {
                ...snapshot.navigation,
                busy: false,
                error,
                revision: snapshot.navigation.revision + 1
            };
            _publish();
            throw Object.assign(new Error("Fixture view operation failed"), error);
        }
        snapshot.navigation = { ...snapshot.navigation, error: null };
        if (request.operation.kind === "back")
        {
            snapshot.navigation = { ...snapshot.navigation, revision: snapshot.navigation.revision + 1, current: null };
            _publish();
        }
        const current = snapshot.navigation.current;
        if (
            localizedViews[variant ?? ""]?.kind === "list"
            && current?.view.kind === "list"
            && request.operation.kind === "event"
        )
        {
            const event = request.operation.event;
            const list = { ...current.view.list };
            if (event.kind === "searchChanged")
            {
                list.search_text = event.text;
            }
            if (event.kind === "filterChanged" && list.filter)
            {
                list.filter = { ...list.filter, selected_value: event.value };
            }
            if (event.kind === "selectionChanged")
            {
                const item = list.sections.flatMap(section => section.items)[event.index];
                if (item)
                {
                    list.selection = { index: event.index, item };
                }
            }
            snapshot.navigation = {
                ...snapshot.navigation,
                revision: snapshot.navigation.revision + 1,
                current: { ...current, revision: current.revision + 1, view: { kind: "list", list } }
            };
            _publish();
        }
        return {
            viewRevision: snapshot.navigation.current?.revision ?? 1,
            navigationRevision: snapshot.navigation.revision
        };
    },
    dismissLauncher: async () =>
    {
        document.documentElement.dataset.dismissed = "true";
    },
    openSettings: async extensionId =>
    {
        navigateSettings(extensionId);
        location.assign(`?surface=settings${extensionId ? `&extension=${encodeURIComponent(extensionId)}` : ""}`);
    },
    readKeyboardLayout: async () => ({ KeyK: "T" }),
    readContextMenu: async request =>
    {
        const target = request.target;
        if (variant !== "shortcuts" || target.kind !== "search")
        {
            return [];
        }
        return results.find(result => result.entryId === target.entryId)?.actions.filter(action => action.id !== "open")
            ?? [];
    },
    invokeContextMenu: async () => null,
    readText: async request =>
    {
        if (variant === "text-read-error")
        {
            document.documentElement.dataset.textReads = String(
                Number(document.documentElement.dataset.textReads ?? 0) + 1
            );
            if (textReadAvailable)
            {
                return { ...request.read, value: "B" };
            }
            const errorResources = new ResourceSet(clipboardResources);
            throw Object.assign(new Error("Text fixture rejected the read"), {
                kind: "extension",
                error: {
                    error: {
                        code: "text_read_failed",
                        message: { message: "error-text-read" },
                        detail: "EACCES: /Example/私密文件.txt"
                    },
                    messages: {
                        en: errorResources.format("en", { message: "error-text-read" }, "macos", 2048),
                        "zh-Hans": errorResources.format("zh-Hans", { message: "error-text-read" }, "macos", 2048)
                    }
                }
            });
        }
        throw new Error("Fixture has only one source chunk");
    },
    readConfigurationMenu: async () => ({
        revision: 1,
        groups: [
            {
                id: "appearance",
                title: { message: "group-appearance" },
                items: [{
                    kind: "toggle",
                    title: { message: "setting-clipboard-syntax-highlighting-title" },
                    key: "clipboard.syntaxHighlighting",
                    checked: syntaxHighlighting
                }]
            },
            {
                id: "danger-zone",
                title: { message: "group-danger-zone" },
                items: [{ kind: "action", action: clearAction }]
            }
        ]
    }),
    invokeConfigurationMenu: async (_request, operation) =>
    {
        if (operation.kind === "set")
        {
            const failure = configurationFailure;
            configurationFailure = undefined;
            if (failure)
            {
                throw Object.assign(new Error("Configuration fixture rejected the change"), failure);
            }
            syntaxHighlighting = operation.value;
        }
        else if (operation.confirmed)
        {
            document.documentElement.dataset.confirmed = "true";
        }
    }
};

export function allowTextRead(): void
{
    textReadAvailable = true;
}

export function failNextConfigurationSave(): void
{
    configurationFailure = { code: "saveFailed", detail: "fixture disk is read-only" };
}

export function changeLauncherLanguage(locale: "en" | "zh-Hans"): void
{
    snapshot.localization = { preference: locale, locale, revision: snapshot.localization.revision + 1 };
    _publish(false);
}

export function setCatalogWarning(enabled: boolean): void
{
    const applicationBundle = new ResourceSet(applicationResources);
    const message = { message: "error-refresh-catalog" };
    snapshot.warnings = enabled
        ? [
            { extensionId: null, error: { kind: "host", code: "hostServicesUnavailable" } },
            {
                extensionId: applicationScope.extensionId,
                error: {
                    kind: "extension",
                    error: {
                        error: {
                            code: "refresh_failed",
                            message,
                            detail: Array.from({ length: 200 }, (_, index) => `EACCES: /Example/应用/${index}`).join(
                                "\n"
                            )
                        },
                        messages: {
                            en: applicationBundle.format("en", message, "macos", 2048, false),
                            "zh-Hans": applicationBundle.format("zh-Hans", message, "macos", 2048, false)
                        }
                    }
                }
            }
        ]
        : [];
    _publish(false);
}

function _publish(includeResults = true): void
{
    snapshot.revision++;
    const next = structuredClone(snapshot);
    // The real bridge retains the previous array when the host omits an unchanged result window.
    publishedResults = includeResults ? next.results : publishedResults;
    next.results = publishedResults;
    void listener?.(next);
}

export function completeViewFailure(): void
{
    finishViewFailure?.();
    finishViewFailure = undefined;
}
