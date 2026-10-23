import { operationErrorText, operationFailure } from "./operation-errors";
import { MAX_LIFECYCLE_DIAGNOSTIC_BYTES } from "../generated/localization-limits";

import type { LifecycleError } from "../generated/LifecycleError";
import type { LifecycleFailure } from "../generated/LifecycleFailure";
import type { LocalizedText } from "../generated/LocalizedText";
import type { SupportedLocale } from "../generated/SupportedLocale";

/**
 * Retain an admitted cause and bounded process output from a lifecycle command.
 * @param value Rejection received from the typed bridge
 */
export function lifecycleFailure(value: unknown): LifecycleFailure
{
    if (value && typeof value === "object" && "error" in value)
    {
        const diagnostic = "diagnostic" in value ? value.diagnostic : undefined;
        if (
            (diagnostic === undefined || (typeof diagnostic === "string"
                && diagnostic.length <= MAX_LIFECYCLE_DIAGNOSTIC_BYTES
                && new TextEncoder().encode(diagnostic).byteLength <= MAX_LIFECYCLE_DIAGNOSTIC_BYTES))
            && value.error !== null && typeof value.error === "object"
        )
        {
            return {
                error: operationFailure(value.error, "extensionLifecycleFailed"),
                ...(diagnostic === undefined ? {} : { diagnostic })
            };
        }
    }
    return { error: operationFailure({}, "extensionLifecycleFailed") };
}

/**
 * Format the current explanation without translating supplemental process diagnostics.
 * @param failure Retained lifecycle cause
 * @param locale Current host locale
 * @param text Current host formatter
 */
export function lifecycleFailureText(
    failure: LifecycleFailure,
    locale: SupportedLocale,
    text: (value: LocalizedText) => string
): string
{
    const retained = lifecycleFailure(failure);
    const explanation = operationErrorText(retained.error, locale, text);
    return retained.diagnostic ? `${explanation}\n${retained.diagnostic}` : explanation;
}

/**
 * Present a bounded recovery history while preserving the separate original and later causes.
 * @param error Authoritative lifecycle metadata
 * @param locale Current host locale
 * @param text Current host formatter
 */
export function lifecycleErrorText(
    error: LifecycleError,
    locale: SupportedLocale,
    text: (value: LocalizedText) => string
): string
{
    const format = (failure: LifecycleFailure) => lifecycleFailureText(failure, locale, text);
    switch (error.phase)
    {
        case "failed":
            return format(error.failure);
        case "restarting":
            return `${text({ message: "error-lifecycle-restarting" })}\n${format(error.failure)}`;
        case "stopping":
            return `${text({ message: "error-lifecycle-stopping" })}\n${format(error.failure)}`;
        case "restartFailed":
        case "restartLimitReached":
            return [
                text({
                    message: error.phase === "restartFailed"
                        ? "error-lifecycle-restart-failed"
                        : "error-lifecycle-restart-limit"
                }),
                `${text({ message: "error-lifecycle-original" })}\n${format(error.original)}`,
                `${text({ message: "error-lifecycle-restarted" })}\n${format(error.failure)}`
            ].join("\n\n");
        case "recoveryStopped":
            if (error.failures.length < 1 || error.failures.length > 2)
            {
                return operationErrorText(operationFailure({}, "extensionLifecycleFailed"), locale, text);
            }
            return [
                text({ message: "error-lifecycle-recovery-stopped" }),
                `${text({ message: "error-lifecycle-original" })}\n${format(error.original)}`,
                text({ message: "error-lifecycle-stop" }),
                ...error.failures.map(format)
            ].join("\n\n");
    }
}
