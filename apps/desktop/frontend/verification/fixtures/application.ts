import { applicationScope } from "./localization";
import applicationIcon from "../../../../extensions/built-in/application/assets/icon.png?url";
import { configurationProperty } from "../../fixtures/configuration";

import type { ExtensionSettings } from "../../src/generated/ExtensionSettings";

const key = "application.builtin.macos.user-applications";
const values = { [key]: false };

export const application: ExtensionSettings = {
    id: applicationScope.extensionId,
    name: { message: "extension-name" },
    resourceScope: applicationScope,
    icon: "assets/icon.png",
    iconUrl: applicationIcon,
    enabled: true,
    pending: false,
    state: "ready",
    instanceId: 1,
    lifecycleError: null,
    configurationError: null,
    configuration: {
        extensionId: applicationScope.extensionId,
        revision: 1,
        values,
        saved: values,
        effective: { [key]: true },
        contribution: {
            title: { message: "extension-name" },
            quickMenu: [],
            actions: [],
            properties: {
                [key]: configurationProperty({
                    title: { message: "setting-application-builtin-macos-user-applications-title" },
                    description: { message: "setting-application-builtin-macos-user-applications-description" },
                    type: "boolean",
                    default: true,
                    persistence: "beforeApply"
                })
            }
        }
    },
    application: {
        requestId: 1,
        extensionId: applicationScope.extensionId,
        resourceScope: applicationScope,
        key,
        result: {
            status: "running",
            progress: { label: { message: "progress-scanning-sources" }, completed: 1, total: 3 }
        }
    }
};
