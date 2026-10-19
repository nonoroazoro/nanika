import { MAX_CONFIGURATION_STRING_BYTES } from "../generated/limits";

import type { ConfigurationSchema } from "../generated/ConfigurationSchema";
import type { JsonValue } from "../generated/serde_json/JsonValue";

const encoder = new TextEncoder();

export function stringError(
    schema: ConfigurationSchema,
    value: JsonValue | undefined
): string | null
{
    const text = typeof value === "string" ? value : "";
    const maximum = schema.maxLength ?? MAX_CONFIGURATION_STRING_BYTES;
    return encoder.encode(text).byteLength > maximum ? `Enter at most ${maximum} UTF-8 bytes.` : null;
}
