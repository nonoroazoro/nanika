import type { ConfigurationSchema } from "../generated/ConfigurationSchema";
import type { JsonValue } from "../generated/serde_json/JsonValue";

export function integerError(schema: ConfigurationSchema, value: JsonValue | undefined): string | null
{
    const text = typeof value === "number" || typeof value === "string" ? String(value) : "";
    const number = Number(text);
    if (!/^-?\d+$/.test(text) || !Number.isSafeInteger(number))
    {
        return "Enter a whole number.";
    }
    if (schema.minimum !== null && schema.minimum !== undefined && number < schema.minimum)
    {
        return `Enter ${schema.minimum} or more.`;
    }
    if (schema.maximum !== null && schema.maximum !== undefined && number > schema.maximum)
    {
        return `Enter ${schema.maximum} or less.`;
    }
    if (number % (schema.multipleOf ?? 1) !== 0)
    {
        return `Enter a multiple of ${schema.multipleOf}.`;
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
