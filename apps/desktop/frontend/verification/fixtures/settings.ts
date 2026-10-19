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
        buttonLabel: "Clear",
        description: "Permanently delete all clipboard history. This cannot be undone.",
        action: {
            id: "clipboard.clear",
            title: "Clear history",
            style: "destructive",
            enabled: true,
            allow_default_execution: false,
            confirmation_title: "Clear all history?",
            confirmation_description: "All clipboard history will be permanently deleted. This cannot be undone.",
            group: "Danger zone"
        }
    }],
    properties: {
        "clipboard.syntaxHighlighting": configurationProperty({
            title: "Syntax highlighting",
            description: "Color recognized code and Markdown in text previews.",
            type: "boolean",
            group: "Appearance",
            default: true,
            persistence: "afterApply"
        }),
        "clipboard.maxEntries": configurationProperty({
            title: "Maximum history entries",
            description: "Maximum number of clipboard history entries retained locally.",
            type: "integerLimit",
            group: "History retention",
            default: { unlimited: false, value: 50 },
            minimum: 1,
            maximum: 5000,
            multipleOf: 1,
            order: 1,
            persistence: "beforeApply"
        }),
        "clipboard.maxAgeDays": configurationProperty({
            title: "Maximum age in days",
            description: "Maximum age of clipboard history retained locally.",
            type: "integerLimit",
            group: "History retention",
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
            group: "Storage",
            default: ["/Example/Clipboard"],
            persistence: "beforeApply",
            order: 3
        })
    }
};

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
    name: "Clipboard History",
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
    navigation: { revision: 1, extensionId: "com.nanika.clipboard" },
    lifecycleRevision: 1,
    maximized: false,
    version: "0.1.0",
    general: { formatVersion: 1, launcherShortcut: "Control+Space", theme: "system", hideOnBlur: true },
    generalSections: [{
        key: "appearance",
        title: "Appearance",
        fields: [{ key: "theme", title: "Theme", description: null }]
    }],
    extensions: [clipboard]
};
