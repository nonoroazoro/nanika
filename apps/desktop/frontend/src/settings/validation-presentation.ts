import type { SettingsValidationError } from "./SettingsValidationError";
import type { LocalizedText } from "../generated/LocalizedText";

const messages: Record<SettingsValidationError["code"], string> = {
    unknownSetting: "validation-unknown-setting",
    wholeNumber: "validation-whole-number",
    integerMinimum: "validation-integer-minimum",
    integerMaximum: "validation-integer-maximum",
    integerMultiple: "validation-integer-multiple",
    integerLimit: "validation-integer-limit",
    textRequired: "validation-text-required",
    textBytes: "validation-text-bytes",
    booleanRequired: "validation-boolean-required",
    arrayRequired: "validation-array-required",
    arrayLength: "validation-array-length",
    objectRequired: "validation-object-required"
};

/**
 * Resolve a retained validation result without validating or modifying the draft again.
 * @param error Structured schema failure, or null for a valid input
 * @param text Current host presentation formatter
 */
export function validationErrorText(
    error: SettingsValidationError | null,
    text: (value: LocalizedText) => string
): string
{
    if (!error)
    {
        return "";
    }
    const explanation = text({
        message: messages[error.code],
        ...("constraint" in error ? { args: { value: error.constraint } } : {})
    });
    return error.path.reduceRight<string>((message, part) =>
    {
        const field = typeof part === "number"
            ? text({ message: "validation-entry", args: { index: part + 1 } })
            : part;
        return text({ message: "validation-field", args: { field, error: message } });
    }, explanation);
}
