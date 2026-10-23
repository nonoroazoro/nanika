import { clipboardScope } from "./localization";
import clipboardIcon from "../../../../extensions/built-in/clipboard/assets/icon.png?url";
import { configurationProperty, configurationSchema } from "../../fixtures/configuration";

import type { ConfigurationContribution } from "../../src/generated/ConfigurationContribution";
import type { ExtensionSettings } from "../../src/generated/ExtensionSettings";
import type { RuntimeExtensionConfiguration } from "../../src/generated/RuntimeExtensionConfiguration";
import type { SettingsSnapshot } from "../../src/generated/SettingsSnapshot";

export const configuration: ConfigurationContribution = {
    title: "Clipboard History",
    quickMenu: [],
    actions: [{
        buttonLabel: { message: "setting-clipboard-clear-button-label" },
        description: { message: "setting-clipboard-clear-description" },
        action: {
            id: "clipboard.clear",
            title: { message: "action-clipboard-clear-title" },
            style: "destructive",
            enabled: true,
            allow_default_execution: false,
            confirmation_title: { message: "action-clipboard-clear-confirmation-title" },
            confirmation_description: { message: "action-clipboard-clear-confirmation-description" },
            group: { id: "danger-zone", title: { message: "group-danger-zone" } }
        }
    }],
    properties: {
        "clipboard.syntaxHighlighting": configurationProperty({
            title: { message: "setting-clipboard-syntax-highlighting-title" },
            description: { message: "setting-clipboard-syntax-highlighting-description" },
            type: "boolean",
            group: { id: "appearance", title: { message: "group-appearance" } },
            default: true,
            persistence: "afterApply"
        }),
        "clipboard.maxEntries": configurationProperty({
            title: { message: "setting-clipboard-max-entries-title" },
            description: { message: "setting-clipboard-max-entries-description" },
            type: "integerLimit",
            group: { id: "history-retention", title: { message: "group-history-retention" } },
            default: { unlimited: false, value: 50 },
            minimum: 1,
            maximum: 5000,
            multipleOf: 1,
            order: 1,
            persistence: "beforeApply"
        }),
        "clipboard.maxAgeDays": configurationProperty({
            title: { message: "setting-clipboard-max-age-days-title" },
            description: { message: "setting-clipboard-max-age-days-description" },
            type: "integerLimit",
            group: { id: "history-retention", title: { message: "group-history-retention" } },
            default: { unlimited: false, value: 7 },
            minimum: 1,
            maximum: 3650,
            multipleOf: 1,
            order: 2,
            persistence: "beforeApply"
        }),
        directories: configurationProperty({
            title: "Folders",
            type: "array",
            maxItems: 16,
            items: configurationSchema({ type: "string", format: "directory" }),
            group: { id: "storage", title: "Storage" },
            default: ["/Example/Clipboard"],
            persistence: "beforeApply",
            order: 3
        })
    }
};

if (new URLSearchParams(location.search).get("state") === "validation")
{
    configuration.properties = {
        "fixture.count": configurationProperty({
            title: "Sample count",
            type: "integer",
            minimum: 3,
            maximum: 9,
            multipleOf: 3,
            default: 3,
            persistence: "beforeApply",
            order: 1
        }),
        "fixture.text": configurationProperty({
            title: "Sample text",
            type: "string",
            maxLength: 4,
            default: "ok",
            persistence: "beforeApply",
            order: 2
        }),
        "fixture.options": configurationProperty({
            title: "Structured options",
            type: "array",
            maxItems: 30,
            items: configurationSchema({
                type: "object",
                properties: {
                    count: configurationSchema({ type: "integer", minimum: 3, maximum: 9, multipleOf: 3 }),
                    tag: configurationSchema({ type: "string", maxLength: 4 })
                },
                required: ["count"]
            }),
            default: Array.from({ length: 21 }, () => ({ count: 3 })),
            persistence: "beforeApply",
            order: 3
        })
    };
}

const values = Object.fromEntries(
    Object.entries(configuration.properties).map(([key, property]) => [key, property.default])
);

export const currentConfiguration: RuntimeExtensionConfiguration = {
    extensionId: "com.nanika.clipboard",
    revision: 1,
    values,
    saved: structuredClone(values),
    effective: structuredClone(values),
    contribution: configuration
};

export const clipboard: ExtensionSettings = {
    id: "com.nanika.clipboard",
    name: { message: "extension-name" },
    resourceScope: clipboardScope,
    icon: "assets/icon.png",
    iconUrl: clipboardIcon,
    enabled: true,
    pending: false,
    state: "ready",
    instanceId: 1,
    lifecycleError: null,
    configurationError: null,
    application: null,
    configuration: currentConfiguration
};

export const settings: SettingsSnapshot = {
    sessionId: 1,
    platform: new URLSearchParams(location.search).get("platform") === "windows" ? "windows" : "macos",
    localization: { preference: "system", locale: "en", revision: 1 },
    navigation: { revision: 1, extensionId: "com.nanika.clipboard" },
    lifecycleRevision: 1,
    maximized: false,
    version: "0.1.0",
    general: {
        formatVersion: 1,
        launcherShortcut: "Control+Space",
        theme: "system",
        hideOnBlur: true,
        language: "system"
    },
    generalSections: [{
        key: "launcher",
        title: { message: "settings-launcher" },
        fields: [{
            key: "launcherShortcut",
            title: { message: "settings-open-launcher" },
            description: null
        }, {
            key: "hideOnBlur",
            title: { message: "settings-hide-on-blur" },
            description: null
        }]
    }, {
        key: "appearance",
        title: { message: "settings-appearance" },
        fields: [{ key: "theme", title: { message: "settings-theme" }, description: null }, {
            key: "language",
            title: { message: "settings-language" },
            description: null
        }]
    }, {
        key: "startup",
        title: { message: "settings-startup" },
        fields: [{
            key: "launchAtLogin",
            title: { message: "settings-launch-at-login" },
            description: { message: "settings-launch-at-login-description" }
        }]
    }],
    extensions: [clipboard]
};
