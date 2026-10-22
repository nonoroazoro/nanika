import { clipboard, configuration } from "./settings";
import { navigateSettings } from "./settingsBridge";
import applicationIcon from "../../../../extensions/built-in/application/assets/icon.png?url";

import type { NanikaBridge } from "../../src/bridge/NanikaBridge";
import type { SearchResult } from "../../src/generated/SearchResult";
import type { RootSearchSnapshot } from "../../src/types/RootSearchSnapshot";

const clearAction = configuration.actions[0]?.action;
if (!clearAction)
{
    throw new Error("Clear action fixture is missing");
}
const snippet = "const message = 'Hello, Nanika';\n";
let syntaxHighlighting = true;
const params = new URLSearchParams(location.search);
const variant = params.get("state");
const results: SearchResult[] = ([
    {
        title: "Clipboard History",
        extensionId: "com.nanika.clipboard",
        subtitle: { kind: "label", text: "Clipboard" },
        icon: { kind: "image", url: clipboard.iconUrl },
        entryType: "view"
    },
    {
        title: "Empty Trash",
        extensionId: "com.nanika.system",
        subtitle: { kind: "label", text: "System" },
        icon: { kind: "symbol", name: "trash" },
        entryType: "action"
    },
    {
        title: "Open Trash",
        extensionId: "com.nanika.system",
        subtitle: { kind: "label", text: "System" },
        icon: { kind: "symbol", name: "folder-open" },
        entryType: "action"
    },
    {
        title: "A long application name for overflow and truncation verification",
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
    entryId: String(index),
    actionId: "open",
    actionStyle: index === 1 ? "destructive" : "primary",
    allowDefaultExecution: index !== 1,
    confirmationTitle: index === 1 ? "Empty Trash?" : null,
    confirmationDescription: index === 1 ? "The items in the Trash will be permanently deleted." : null,
    kind: "Extension"
}));
let listener: ((snapshot: RootSearchSnapshot) => void) | undefined;
let snapshot: RootSearchSnapshot = {
    sessionId: 1,
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
    openSession: async callback =>
    {
        listener = callback;
        if (variant === "failure")
        {
            throw new Error("Fixture search unavailable");
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
            version: "0.1.0",
            locale: "en",
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
        const filtered = results.filter(item => item.title.toLowerCase().includes(request.query.toLowerCase()));
        snapshot = {
            ...snapshot,
            query: request.query,
            requestId: request.requestId,
            results: filtered,
            totalResults: filtered.length,
            resultRevision: snapshot.resultRevision + 1
        };
        _publish();
    },
    readResults: async () =>
    {},
    invokeCandidate: async request =>
    {
        if (request.entryId !== "0")
        {
            const dialog = document.querySelector("[role=alertdialog]");
            document.documentElement.dataset.confirmationAtInvoke = JSON.stringify({
                confirmed: request.confirmed,
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
                extensionName: "Clipboard History",
                extensionIconUrl: clipboard.iconUrl,
                instanceId: 1,
                generation: 1,
                viewId: "clipboard.history",
                revision: 1,
                view: {
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
        if (request.operation.kind === "back")
        {
            snapshot.navigation = { ...snapshot.navigation, revision: snapshot.navigation.revision + 1, current: null };
            _publish();
        }
        return { viewRevision: 1, navigationRevision: snapshot.navigation.revision };
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
    readContextMenu: async () => [],
    invokeContextMenu: async () => null,
    readText: async () =>
    {
        throw new Error("Fixture has only one source chunk");
    },
    readConfigurationMenu: async () => ({
        revision: 1,
        groups: [
            {
                title: "Appearance",
                items: [{
                    kind: "toggle",
                    title: "Syntax highlighting",
                    key: "clipboard.syntaxHighlighting",
                    checked: syntaxHighlighting
                }]
            },
            { title: "Danger zone", items: [{ kind: "action", action: clearAction }] }
        ]
    }),
    invokeConfigurationMenu: async (_request, operation) =>
    {
        if (operation.kind === "set")
        {
            syntaxHighlighting = operation.value;
        }
        else if (operation.confirmed)
        {
            document.documentElement.dataset.confirmed = "true";
        }
    }
};

function _publish(): void
{
    snapshot.revision++;
    listener?.(structuredClone(snapshot));
}
