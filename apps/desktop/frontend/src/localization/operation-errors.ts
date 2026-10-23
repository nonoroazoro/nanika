import { validOutput } from "./output";
import {
    MAX_ERROR_DETAIL_BYTES,
    MAX_ERROR_MESSAGE_CHARS,
    MAX_MESSAGE_ARGUMENT_BYTES,
    MAX_MESSAGE_ARGUMENTS
} from "../generated/localization-limits";

import type { LocalizedMessage } from "../generated/LocalizedMessage";
import type { LocalizedText } from "../generated/LocalizedText";
import type { OperationError } from "../generated/OperationError";
import type { OperationErrorCode } from "../generated/OperationErrorCode";
import type { ProtocolError } from "../generated/ProtocolError";
import type { SupportedLocale } from "../generated/SupportedLocale";

const messages = {
    extensionUnavailable: { message: "error-operation-extension-unavailable" },
    cancelled: { message: "error-operation-cancelled" },
    extensionTerminated: { message: "error-operation-extension-terminated" },
    extensionIo: { message: "error-operation-extension-io" },
    extensionProtocol: { message: "error-operation-extension-protocol" },
    textReadFailed: { message: "error-operation-text-read-failed" },
    textReadTimeout: { message: "error-operation-text-read-timeout" },
    textSourceChanged: { message: "error-operation-text-source-changed" },
    invalidTextRead: { message: "error-operation-invalid-text-read" },
    sessionUnavailable: { message: "error-operation-session-unavailable" },
    runtimeUnavailable: { message: "error-operation-runtime-unavailable" },
    runtimeStartupFailed: { message: "error-operation-runtime-startup-failed" },
    builtInInventoryUnavailable: { message: "error-operation-built-in-inventory-unavailable" },
    hostConfigurationUnavailable: { message: "error-operation-host-configuration-unavailable" },
    extensionRegistryUnavailable: { message: "error-operation-extension-registry-unavailable" },
    hostStorageUnavailable: { message: "error-operation-host-storage-unavailable" },
    searchUnavailable: { message: "error-operation-search-unavailable" },
    startupDiagnosticUnavailable: { message: "error-operation-startup-diagnostic-unavailable" },
    invalidSearchRequest: { message: "error-operation-invalid-search-request" },
    searchQueryTooLong: { message: "error-operation-search-query-too-long" },
    searchSubmissionFailed: { message: "error-operation-search-submission-failed" },
    settingsSearchFailed: { message: "error-operation-settings-search-failed" },
    settingsReadFailed: { message: "error-operation-settings-read-failed" },
    settingsOpenFailed: { message: "error-operation-settings-open-failed" },
    settingsReadyFailed: { message: "error-operation-settings-ready-failed" },
    windowDragFailed: { message: "error-operation-window-drag-failed" },
    windowMinimizeFailed: { message: "error-operation-window-minimize-failed" },
    windowResizeFailed: { message: "error-operation-window-resize-failed" },
    windowCloseFailed: { message: "error-operation-window-close-failed" },
    startupReadFailed: { message: "error-operation-startup-read-failed" },
    startupWriteUnconfirmed: { message: "error-operation-startup-write-unconfirmed" },
    invalidHostPreferences: { message: "error-operation-invalid-host-preferences" },
    invalidShortcut: { message: "error-operation-invalid-shortcut" },
    shortcutModifiersRequired: { message: "settings-shortcut-modifiers" },
    shortcutRegistrationFailed: { message: "error-operation-shortcut-registration-failed" },
    shortcutReleaseFailed: { message: "error-operation-shortcut-release-failed" },
    shortcutCleanupFailed: { message: "error-operation-shortcut-cleanup-failed" },
    preferencesSaveFailed: { message: "error-operation-preferences-save-failed" },
    preferencesRollbackFailed: { message: "error-operation-preferences-rollback-failed" },
    preferencesConfirmationFailed: { message: "error-operation-preferences-confirmation-failed" },
    preferencesUnconfirmed: { message: "error-operation-preferences-unconfirmed" },
    nativeLanguageUpdateFailed: { message: "error-operation-native-language-update-failed" },
    languageUpdateFailed: { message: "error-operation-language-update-failed" },
    directorySettingUnavailable: { message: "error-operation-directory-setting-unavailable" },
    directoryPickerFailed: { message: "error-operation-directory-picker-failed" },
    directoryUnavailable: { message: "error-operation-directory-unavailable" },
    directoryPathInvalid: { message: "error-operation-directory-path-invalid" },
    shortcutRecordingFailed: { message: "error-operation-shortcut-recording-failed" },
    sessionOpenFailed: { message: "error-operation-session-open-failed" },
    searchAcknowledgementFailed: { message: "error-operation-search-acknowledgement-failed" },
    invalidResultRange: { message: "error-operation-invalid-result-range" },
    localizationBusy: { message: "error-operation-localization-busy" },
    localizationScopeInvalid: { message: "error-operation-localization-scope-invalid" },
    localizationResourcesUnavailable: { message: "error-operation-localization-resources-unavailable" },
    localizationPackageChanged: { message: "error-operation-localization-package-changed" },
    localizationInstanceRetired: { message: "error-operation-localization-instance-retired" },
    localizationReadFailed: { message: "localization-load-failed" },
    searchChanged: { message: "error-operation-search-changed" },
    actionUnavailable: { message: "error-operation-action-unavailable" },
    operationPending: { message: "error-operation-operation-pending" },
    invocationFailed: { message: "error-operation-invocation-failed" },
    invocationUnconfirmed: { message: "error-operation-invocation-unconfirmed" },
    settingsNavigationUnsupported: { message: "error-operation-settings-navigation-unsupported" },
    navigationFailed: { message: "error-operation-navigation-failed" },
    viewOperationFailed: { message: "error-operation-view-operation-failed" },
    viewOperationUnconfirmed: { message: "error-operation-view-operation-unconfirmed" },
    viewChanged: { message: "error-operation-view-changed" },
    viewInteractionUnavailable: { message: "error-operation-view-interaction-unavailable" },
    confirmationChanged: { message: "error-operation-confirmation-changed" },
    navigationDepthReached: { message: "error-operation-navigation-depth-reached" },
    configurationChanged: { message: "error-operation-configuration-changed" },
    configurationPropertyUnavailable: { message: "error-operation-configuration-property-unavailable" },
    invalidConfiguration: { message: "error-operation-invalid-configuration" },
    configurationSubmissionFailed: { message: "error-operation-configuration-submission-failed" },
    configurationUnconfirmed: { message: "error-operation-configuration-unconfirmed" },
    refreshUnconfirmed: { message: "error-operation-refresh-unconfirmed" },
    hostServicesUnavailable: { message: "error-operation-host-services-unavailable" },
    extensionStorageUnavailable: { message: "error-operation-extension-storage-unavailable" },
    extensionPackageUnavailable: { message: "error-operation-extension-package-unavailable" },
    extensionIconUnavailable: { message: "error-operation-extension-icon-unavailable" },
    extensionConfigurationUnavailable: { message: "error-operation-extension-configuration-unavailable" },
    extensionExited: { message: "error-operation-extension-exited" },
    extensionStopUnconfirmed: { message: "error-operation-extension-stop-unconfirmed" },
    extensionStillStopping: { message: "error-operation-extension-still-stopping" },
    extensionRegistrySaveFailed: { message: "error-operation-extension-registry-save-failed" },
    extensionLifecycleUnconfirmed: { message: "error-operation-extension-lifecycle-unconfirmed" },
    extensionLifecycleFailed: { message: "error-operation-extension-lifecycle-failed" },
    catalogRetirementFailed: { message: "error-operation-catalog-retirement-failed" },
    applicationShuttingDown: { message: "error-operation-application-shutting-down" }
} satisfies Record<OperationErrorCode, LocalizedMessage>;

/**
 * Retain a bounded command rejection without interpreting diagnostic text as a code.
 * @param value Rejection from the typed bridge or a local transport error
 * @param fallback Explanation for a local failure without a structured code
 */
export function operationFailure(value: unknown, fallback: OperationErrorCode): OperationError
{
    const retained = _retainedFailure(value, true);
    if (retained)
    {
        return retained;
    }
    const data = _record(value);
    if (data && (!(value instanceof Error) || data.kind !== undefined))
    {
        return { kind: "host", code: "extensionProtocol", detail: "Invalid structured error presentation." };
    }
    const detail = value instanceof Error ? value.message : String(value);
    return validErrorDetail(detail)
        ? { kind: "host", code: fallback, detail }
        : { kind: "host", code: "extensionProtocol", detail: "Error diagnostic exceeds the presentation contract." };
}

/**
 * Select retained extension text or current host FTL while preserving original diagnostics.
 * @param error Validated operation failure
 * @param locale Current host locale
 * @param text Current host formatter
 */
export function operationErrorText(
    error: OperationError,
    locale: SupportedLocale,
    text: (value: LocalizedText) => string
): string
{
    if (error.kind === "cleanup")
    {
        if (error.primary.kind === "cleanup" || error.cleanup.kind === "cleanup")
        {
            return text(messages.extensionProtocol);
        }
        return `${operationErrorText(error.primary, locale, text)}\n\n${
            text({ message: "error-operation-cleanup-failed" })
        }\n${operationErrorText(error.cleanup, locale, text)}`;
    }
    const explanation = error.kind === "host" ? text(messages[error.code]) : error.error.messages[locale];
    const detail = error.kind === "host" ? error.detail : error.error.error.detail;
    if (!explanation || !validOutput(explanation, MAX_ERROR_MESSAGE_CHARS, false) || !validErrorDetail(detail))
    {
        return text(messages.extensionProtocol);
    }
    return detail ? `${explanation}\n${detail}` : explanation;
}

/**
 * Check the shared diagnostic byte bound before retaining or presenting original text.
 * @param value Optional diagnostic received from a host or extension boundary
 */
export function validErrorDetail(value: unknown): value is string | undefined
{
    return value === undefined
        || (typeof value === "string" && value.length <= MAX_ERROR_DETAIL_BYTES && !value.includes("\0")
            && new TextEncoder().encode(value).byteLength <= MAX_ERROR_DETAIL_BYTES);
}

function _retainedFailure(value: unknown, allowCleanup: boolean): OperationError | null
{
    const data = _record(value);
    if (
        data?.kind === "host" && typeof data.code === "string" && Object.hasOwn(messages, data.code)
        && validErrorDetail(data.detail)
    )
    {
        return {
            kind: "host",
            code: data.code as OperationErrorCode,
            ...(data.detail === undefined ? {} : { detail: data.detail })
        };
    }
    const prepared = _record(data?.error);
    const translations = _record(prepared?.messages);
    if (data?.kind === "extension" && _protocolError(prepared?.error) && translations)
    {
        const english = translations.en;
        const chinese = translations["zh-Hans"];
        if (
            typeof english === "string" && typeof chinese === "string"
            && validOutput(english, MAX_ERROR_MESSAGE_CHARS, false)
            && validOutput(chinese, MAX_ERROR_MESSAGE_CHARS, false)
        )
        {
            return {
                kind: "extension",
                error: { error: structuredClone(prepared.error), messages: { en: english, "zh-Hans": chinese } }
            };
        }
    }
    if (allowCleanup && data?.kind === "cleanup")
    {
        // A cleanup group contains exactly two ordinary failures, never recursive groups.
        const primary = _retainedFailure(data.primary, false);
        const cleanup = _retainedFailure(data.cleanup, false);
        if (primary && cleanup)
        {
            return { kind: "cleanup", primary, cleanup };
        }
    }
    return null;
}

function _record(value: unknown): Record<string, unknown> | null
{
    return value !== null && typeof value === "object" && !Array.isArray(value)
        ? value as Record<string, unknown>
        : null;
}

function _protocolError(value: unknown): value is ProtocolError
{
    const error = _record(value);
    if (
        !error || Object.keys(error).some(key => key !== "code" && key !== "message" && key !== "detail")
        || typeof error.code !== "string" || !/^[A-Za-z0-9_.-]{1,128}$/.test(error.code)
        || !validErrorDetail(error.detail)
    )
    {
        return false;
    }
    if (typeof error.message === "string")
    {
        return validOutput(error.message, MAX_ERROR_MESSAGE_CHARS, false);
    }
    const reference = _record(error.message);
    if (
        !reference || Object.keys(reference).some(key => key !== "message" && key !== "args")
        || !_identifier(reference.message)
    )
    {
        return false;
    }
    const args = reference.args === undefined ? {} : _record(reference.args);
    return args !== null && Object.keys(args).length <= MAX_MESSAGE_ARGUMENTS
        && Object.entries(args).every(_argument);
}

function _argument([name, value]: [string, unknown]): boolean
{
    if (!_identifier(name) || name === "platform")
    {
        return false;
    }
    return typeof value === "number"
        ? Number.isSafeInteger(value)
        : typeof value === "string" && value.length <= MAX_MESSAGE_ARGUMENT_BYTES && !value.includes("\0")
            && new TextEncoder().encode(value).byteLength <= MAX_MESSAGE_ARGUMENT_BYTES;
}

function _identifier(value: unknown): value is string
{
    return typeof value === "string" && /^[A-Za-z][A-Za-z0-9_-]{0,127}$/.test(value);
}
