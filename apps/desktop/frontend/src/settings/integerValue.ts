import type { SettingsValidationError } from "./SettingsValidationError";
import type { ConfigurationSchema } from "../generated/ConfigurationSchema";
import type { JsonValue } from "../generated/serde_json/JsonValue";

export function integerError(schema: ConfigurationSchema, value: JsonValue | undefined): SettingsValidationError | null
{
    const text = typeof value === "number" || typeof value === "string" ? String(value) : "";
    const number = Number(text);
    if (!/^-?\d+$/.test(text) || !Number.isSafeInteger(number))
    {
        return { code: "wholeNumber", path: [] };
    }
    if (schema.minimum !== null && schema.minimum !== undefined && number < schema.minimum)
    {
        return { code: "integerMinimum", constraint: schema.minimum, path: [] };
    }
    if (schema.maximum !== null && schema.maximum !== undefined && number > schema.maximum)
    {
        return { code: "integerMaximum", constraint: schema.maximum, path: [] };
    }
    if (number % (schema.multipleOf ?? 1) !== 0)
    {
        return { code: "integerMultiple", constraint: schema.multipleOf ?? 1, path: [] };
    }
    return null;
}

export function normalizeSetting(schema: ConfigurationSchema, value: JsonValue): JsonValue
{
    if (schema.type === "integerLimit")
    {
        const limit = integerLimitDraft(value);
        return limit && integerError(schema, limit.value) === null
            ? { unlimited: limit.unlimited, value: Number(limit.value) }
            : value;
    }
    if (schema.type === "integer" && value !== null && integerError(schema, value) === null)
    {
        return Number(value);
    }
    if (schema.type === "array" && Array.isArray(value) && schema.items)
    {
        const itemSchema = schema.items;
        return value.map(item => normalizeSetting(itemSchema, item));
    }
    if (schema.type === "object" && value !== null && typeof value === "object" && !Array.isArray(value))
    {
        return normalizeSettings(schema.properties, value);
    }
    return value;
}

export function normalizeSettings(
    properties: Record<string, ConfigurationSchema>,
    values: Record<string, JsonValue>
): Record<string, JsonValue>
{
    return Object.fromEntries(
        Object.entries(values).map((
            [key, value]
        ) => [key, properties[key] ? normalizeSetting(properties[key], value) : value])
    );
}

/**
 * Read only the explicit two-field shape, preserving unfinished numeric drafts.
 * @param value Stored limit or editor draft
 */
export function integerLimitDraft(value: JsonValue | undefined): { unlimited: boolean; value: number | string; } | null
{
    if (
        !value || typeof value !== "object" || Array.isArray(value)
        || Object.keys(value).length !== 2 || typeof value.unlimited !== "boolean"
        || (typeof value.value !== "number" && typeof value.value !== "string")
    )
    {
        return null;
    }
    return { unlimited: value.unlimited, value: value.value };
}
