import { clipboardResources } from "./localization";
import { ResourceSet } from "../../src/localization/ResourceSet";

import type { LifecycleFailure } from "../../src/generated/LifecycleFailure";

const resources = new ResourceSet(clipboardResources);
const message = { message: "error-invalid-configuration" };

export const initializationFailure: LifecycleFailure = {
    error: {
        kind: "extension",
        error: {
            error: { code: "invalid_configuration", message, detail: "invalid value: fixture.enabled" },
            messages: {
                en: resources.format("en", message, "macos", 2048, false),
                "zh-Hans": resources.format("zh-Hans", message, "macos", 2048, false)
            }
        }
    },
    diagnostic: Array.from({ length: 200 }, (_, index) => `stderr: original diagnostic ${index}`).join("\n")
};
