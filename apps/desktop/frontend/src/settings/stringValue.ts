import type { ConfigurationSchema, ConfigurationValue } from "../types/Settings";

const encoder = new TextEncoder();

export function stringError(
    schema: ConfigurationSchema,
    value: ConfigurationValue | undefined
): string | null
{
    const text = typeof value === "string" ? value : "";
    const maximum = schema.maxLength ?? 4096;
    return encoder.encode(text).byteLength > maximum ? `Enter at most ${maximum} UTF-8 bytes.` : null;
}
