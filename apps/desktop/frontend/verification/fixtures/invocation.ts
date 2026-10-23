import { clipboardResources } from "./localization";
import { ResourceSet } from "../../src/localization/ResourceSet";

import type { OperationError } from "../../src/generated/OperationError";

const resources = new ResourceSet(clipboardResources);
let finish: (() => void) | undefined;
let rejected = false;

export async function failFirstInvocation(
    messageId: "error-clear-history" | "error-host-service" | "error-open-history"
): Promise<void>
{
    document.documentElement.dataset.actionAttempts = String(
        Number(document.documentElement.dataset.actionAttempts ?? 0) + 1
    );
    if (rejected)
    {
        return;
    }
    await new Promise<void>(resolve =>
    {
        finish = resolve;
    });
    rejected = true;
    const hostService = messageId === "error-host-service";
    const message = hostService
        ? { message: messageId, args: { outcome: "unconfirmed" } }
        : { message: messageId };
    const failure: OperationError = {
        kind: "extension",
        error: {
            error: {
                code: hostService ? "host_service_unconfirmed" : "action_failed",
                message,
                detail: hostService ? "host service closed before replying" : "EACCES: /Example/历史.db"
            },
            messages: {
                en: resources.format("en", message, "macos", 2048, false),
                "zh-Hans": resources.format("zh-Hans", message, "macos", 2048, false)
            }
        }
    };

    throw Object.assign(new Error("Fixture action failed"), structuredClone(failure));
}

export function finishInvocationFailure(): void
{
    finish?.();
    finish = undefined;
}
