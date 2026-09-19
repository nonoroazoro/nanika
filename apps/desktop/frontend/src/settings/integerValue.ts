import type { ConfigurationSchema, ConfigurationValue } from "../types/Settings";

export function integerError(schema: ConfigurationSchema, value: ConfigurationValue | undefined): string | null
{
    if (value === null && schema.allowUnlimited)
    {
        return null;
    }
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

export function normalizeSetting(schema: ConfigurationSchema, value: ConfigurationValue): ConfigurationValue
{
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
    values: Record<string, ConfigurationValue>
): Record<string, ConfigurationValue>
{
    return Object.fromEntries(
        Object.entries(values).map((
            [key, value]
        ) => [key, properties[key] ? normalizeSetting(properties[key], value) : value])
    );
}
