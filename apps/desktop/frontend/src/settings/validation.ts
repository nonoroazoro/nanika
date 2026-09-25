import { integerError, normalizeSetting } from "./integerValue";
import { stringError } from "./stringValue";

import type { SettingsEditResult } from "./SettingsEditResult";
import type { ConfigurationSchema, ConfigurationValue } from "../types/Settings";

/**
 * Validate the complete draft, including array entries outside the rendered page.
 * Rust remains authoritative for schema and permission validation.
 */
export function settingsError(
    properties: Record<string, ConfigurationSchema>,
    values: Record<string, ConfigurationValue>
): string | null
{
    for (const [key, schema] of Object.entries(properties))
    {
        const error = _valueError(schema, values[key]);
        if (error)
        {
            return `${key}: ${error}`;
        }
    }
    return null;
}

/**
 * Prepare a complete property before no-op detection or operation admission.
 * Schema types decide conversion; numeric-looking string settings remain strings.
 */
export function prepareSetting(
    properties: Record<string, ConfigurationSchema>,
    key: string,
    draft: ConfigurationValue
): SettingsEditResult<ConfigurationValue>
{
    const schema = Object.hasOwn(properties, key) ? properties[key] : undefined;
    if (!schema)
    {
        return { error: "Unknown setting." };
    }
    const error = _valueError(schema, draft);
    return error ? { error: `${key}: ${error}` } : { value: normalizeSetting(schema, draft) };
}

function _valueError(schema: ConfigurationSchema, value: ConfigurationValue | undefined): string | null
{
    if (schema.type === "integer")
    {
        return integerError(schema, value);
    }
    if (schema.type === "string")
    {
        return typeof value === "string" ? stringError(schema, value) : "Enter text.";
    }
    if (schema.type === "boolean")
    {
        return typeof value === "boolean" ? null : "Choose on or off.";
    }
    if (schema.type === "array")
    {
        if (!Array.isArray(value) || !schema.items)
        {
            return "Enter a list.";
        }
        if (value.length > (schema.maxItems ?? 0))
        {
            return `Enter at most ${schema.maxItems ?? 0} entries.`;
        }
        for (const [index, item] of value.entries())
        {
            const error = _valueError(schema.items, item);
            if (error)
            {
                return `Entry ${index + 1}: ${error}`;
            }
        }
    }
    if (schema.type === "object")
    {
        if (!value || typeof value !== "object" || Array.isArray(value))
        {
            return "Enter an object.";
        }
        const included = Object.fromEntries(
            Object.entries(schema.properties).filter(([key]) =>
            {
                return Object.hasOwn(value, key) || schema.required.includes(key);
            })
        );
        return settingsError(included, value);
    }
    return null;
}
