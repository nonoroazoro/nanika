import { integerError, integerLimitDraft, normalizeSetting } from "./integerValue";
import { stringError } from "./stringValue";

import type { SettingsEditResult } from "./SettingsEditResult";
import type { SettingsValidationError } from "./SettingsValidationError";
import type { ConfigurationSchema } from "../generated/ConfigurationSchema";
import type { JsonValue } from "../generated/serde_json/JsonValue";

/**
 * Validate the complete draft, including array entries outside the rendered page.
 * Rust remains authoritative for schema and permission validation.
 */
export function settingsError(
    properties: Record<string, ConfigurationSchema>,
    values: Record<string, JsonValue>
): SettingsValidationError | null
{
    for (const [key, schema] of Object.entries(properties))
    {
        const error = _valueError(schema, values[key]);
        if (error)
        {
            return { ...error, path: [key, ...error.path] };
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
    draft: JsonValue
): SettingsEditResult<JsonValue>
{
    const schema = Object.hasOwn(properties, key) ? properties[key] : undefined;
    if (!schema)
    {
        return { error: { code: "unknownSetting", path: [key] } };
    }
    const error = _valueError(schema, draft);
    return error ? { error: { ...error, path: [key, ...error.path] } } : { value: normalizeSetting(schema, draft) };
}

function _valueError(schema: ConfigurationSchema, value: JsonValue | undefined): SettingsValidationError | null
{
    if (schema.type === "integerLimit")
    {
        const limit = integerLimitDraft(value);
        return limit ? integerError(schema, limit.value) : { code: "integerLimit", path: [] };
    }
    if (schema.type === "integer")
    {
        return integerError(schema, value);
    }
    if (schema.type === "string")
    {
        return typeof value === "string" ? stringError(schema, value) : { code: "textRequired", path: [] };
    }
    if (schema.type === "boolean")
    {
        return typeof value === "boolean" ? null : { code: "booleanRequired", path: [] };
    }
    if (schema.type === "array")
    {
        if (!Array.isArray(value) || !schema.items)
        {
            return { code: "arrayRequired", path: [] };
        }
        if (value.length > (schema.maxItems ?? 0))
        {
            return { code: "arrayLength", constraint: schema.maxItems ?? 0, path: [] };
        }
        for (const [index, item] of value.entries())
        {
            const error = _valueError(schema.items, item);
            if (error)
            {
                return { ...error, path: [index, ...error.path] };
            }
        }
    }
    if (schema.type === "object")
    {
        if (!value || typeof value !== "object" || Array.isArray(value))
        {
            return { code: "objectRequired", path: [] };
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
