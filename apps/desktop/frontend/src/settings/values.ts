import type { ConfigurationSchema } from "../generated/ConfigurationSchema";
import type { JsonValue } from "../generated/serde_json/JsonValue";

export function fieldTitle(key: string): string
{
    const words = key.replace(/([a-z])([A-Z])/g, "$1 $2").replace(/[_-]/g, " ");
    return words.charAt(0).toUpperCase() + words.slice(1);
}

export function emptyValue(schema: ConfigurationSchema): JsonValue
{
    switch (schema.type)
    {
        case "boolean":
            return false;
        case "integerLimit":
        case "integer":
        {
            const step = schema.multipleOf ?? 1;
            const value = schema.minimum != null
                ? Math.ceil(schema.minimum / step) * step
                : Math.floor(Math.min(0, schema.maximum ?? 0) / step) * step;
            return schema.type === "integerLimit" ? { unlimited: false, value } : value;
        }
        case "string":
            return "";
        case "array":
            return [];
        case "object":
            return Object.fromEntries(
                Object.entries(schema.properties)
                    .filter(([key]) => schema.required.includes(key))
                    .map(([key, value]) => [key, emptyValue(value)])
            );
    }
}
