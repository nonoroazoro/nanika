import { hostSettingsErrorText, hostSettingsFailure } from "./host-settings-errors";
import { validationErrorText } from "./validation-presentation";
import { lifecycleFailure, lifecycleFailureText } from "../localization/lifecycle-errors";
import { operationErrorText, operationFailure, validErrorDetail } from "../localization/operation-errors";

import type { SettingsError } from "./SettingsError";
import type { ConfigurationSaveError } from "../generated/ConfigurationSaveError";
import type { LocalizedMessage } from "../generated/LocalizedMessage";
import type { LocalizedText } from "../generated/LocalizedText";
import type { SupportedLocale } from "../generated/SupportedLocale";

const messages = {
    rejected: { message: "error-configuration-rejected" },
    unconfirmed: { message: "error-configuration-unconfirmed" },
    saveFailed: { message: "error-configuration-save-failed" },
    appliedButNotSaved: { message: "error-configuration-applied-not-saved" },
    unavailable: { message: "error-configuration-unavailable" },
    unconfirmedNotSaved: { message: "error-configuration-unconfirmed-not-saved" },
    savedButNotApplied: { message: "error-configuration-saved-not-applied" }
} satisfies Record<ConfigurationSaveError["code"], LocalizedMessage>;

/**
 * Preserve a structured command rejection; never classify translated diagnostic text.
 * @param error Rejection received from the command bridge
 */
export function settingsFailure(error: unknown): SettingsError
{
    if (error && typeof error === "object" && "kind" in error)
    {
        if (error.kind === "HostSettingsFailure")
        {
            return hostSettingsFailure(error);
        }
        return operationFailure(error, "invocationFailed");
    }
    if (error && typeof error === "object" && "error" in error)
    {
        return lifecycleFailure(error);
    }
    if (
        error && typeof error === "object" && "code" in error && typeof error.code === "string"
        && Object.hasOwn(messages, error.code)
    )
    {
        if (error.code === "unavailable")
        {
            return { code: "unavailable" };
        }
        if (
            error.code === "unconfirmedNotSaved" || error.code === "savedButNotApplied"
            || error.code === "rejected" || error.code === "unconfirmed"
        )
        {
            const cause = "cause" in error && error.cause !== null && typeof error.cause === "object"
                ? error.cause
                : {};
            return {
                code: error.code,
                cause: operationFailure(cause, "configurationUnconfirmed")
            };
        }
        if (
            "detail" in error && typeof error.detail === "string"
            && validErrorDetail(error.detail)
        )
        {
            // The recognized code and its required detail are checked before retaining the rejection.
            return { code: error.code, detail: error.detail } as ConfigurationSaveError;
        }
    }
    return operationFailure(error, "invocationUnconfirmed");
}

/**
 * A transport rejection cannot prove whether an accepted configuration change committed.
 * @param error Command rejection or missing terminal result
 */
export function configurationFailure(error: unknown): ConfigurationSaveError
{
    const failure = settingsFailure(error);
    if ("code" in failure && Object.hasOwn(messages, failure.code))
    {
        return failure as ConfigurationSaveError;
    }
    return { code: "unconfirmed", cause: operationFailure(error, "configurationUnconfirmed") };
}

/**
 * Format only the explanation; diagnostic details remain original text.
 * @param error Retained operation failure
 * @param locale Current host locale
 * @param text Current host presentation formatter
 */
export function settingsErrorText(
    error: SettingsError,
    locale: SupportedLocale,
    text: (value: LocalizedText) => string
): string
{
    if ("kind" in error)
    {
        if (error.kind === "HostSettingsFailure")
        {
            return hostSettingsErrorText(error, locale, text);
        }
        return operationErrorText(error, locale, text);
    }
    if ("error" in error)
    {
        return lifecycleFailureText(error, locale, text);
    }
    if ("path" in error)
    {
        return validationErrorText(error, text);
    }
    const explanation = text(messages[error.code]);
    if ("cause" in error)
    {
        return `${explanation}\n${operationErrorText(error.cause, locale, text)}`;
    }
    return "detail" in error && error.detail ? `${explanation}\n${error.detail}` : explanation;
}
