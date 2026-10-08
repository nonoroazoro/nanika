import type { ConfigurationSchema } from "../generated/ConfigurationSchema";
import type { JsonValue } from "../generated/serde_json/JsonValue";

const encoder = new TextEncoder();

export function stringError(
    schema: ConfigurationSchema,
    value: JsonValue | undefined
): string | null
{
    const text = typeof value === "string" ? value : "";
    const maximum = schema.maxLength ?? 4096;
    return encoder.encode(text).byteLength > maximum ? `Enter at most ${maximum} UTF-8 bytes.` : null;
}
