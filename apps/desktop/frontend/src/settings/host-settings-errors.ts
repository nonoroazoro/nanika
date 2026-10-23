import { operationErrorText, operationFailure } from "../localization/operation-errors";

import type { HostSettingsFailure } from "../generated/HostSettingsFailure";
import type { LocalizedText } from "../generated/LocalizedText";
import type { OperationError } from "../generated/OperationError";
import type { SupportedLocale } from "../generated/SupportedLocale";

/**
 * Retain the fixed host preference recovery stages without inferring transaction facts.
 * @param value Host receipt or command rejection
 */
export function hostSettingsFailure(value: unknown): HostSettingsFailure
{
    const failure: HostSettingsFailure = {
        kind: "HostSettingsFailure",
        primary: operationFailure(value, "preferencesUnconfirmed"),
        rollback: null,
        cleanup: null,
        confirmation: null
    };
    if (!value || typeof value !== "object" || !("kind" in value) || value.kind !== "HostSettingsFailure")
    {
        return failure;
    }
    const retained = { ...failure };
    for (const key of ["primary", "rollback", "cleanup", "confirmation"] as const)
    {
        const cause = (value as Record<string, unknown>)[key];
        if (cause === null && key !== "primary")
        {
            continue;
        }
        // These are host transaction stages, never recursive groups or extension-owned messages.
        if (!cause || typeof cause !== "object" || !("kind" in cause) || cause.kind !== "host")
        {
            return failure;
        }
        retained[key] = operationFailure(cause, "preferencesUnconfirmed");
    }
    return retained;
}

/**
 * Present each retained failure using the current locale, preserving original diagnostics.
 * @param failure Host preference and recovery failures
 * @param locale Current host locale
 * @param text Current host formatter
 */
export function hostSettingsErrorText(
    failure: HostSettingsFailure,
    locale: SupportedLocale,
    text: (value: LocalizedText) => string
): string
{
    return [failure.primary, failure.rollback, failure.cleanup, failure.confirmation]
        .filter((error): error is OperationError => error !== null)
        .map(error => operationErrorText(error, locale, text))
        .join("\n\n");
}
