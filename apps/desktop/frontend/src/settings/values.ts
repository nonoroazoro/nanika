import type { ConfigurationSchema, ConfigurationValue } from "../types/Settings";

export function fieldTitle(key: string): string
{
    const words = key.replace(/([a-z])([A-Z])/g, "$1 $2").replace(/[_-]/g, " ");
    return words.charAt(0).toUpperCase() + words.slice(1);
}

export function emptyValue(schema: ConfigurationSchema): ConfigurationValue
{
    switch (schema.type)
    {
        case "boolean":
            return false;
        case "integer":
        {
            const step = schema.multipleOf ?? 1;
            return Math.ceil((schema.minimum ?? 0) / step) * step;
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
