import { MAX_CONFIGURATION_STRING_BYTES } from "../generated/limits";

import type { SettingsValidationError } from "./SettingsValidationError";
import type { ConfigurationSchema } from "../generated/ConfigurationSchema";
import type { JsonValue } from "../generated/serde_json/JsonValue";

const encoder = new TextEncoder();

export function stringError(
    schema: ConfigurationSchema,
    value: JsonValue | undefined
): SettingsValidationError | null
{
    const text = typeof value === "string" ? value : "";
    const maximum = schema.maxLength ?? MAX_CONFIGURATION_STRING_BYTES;
    return encoder.encode(text).byteLength > maximum ? { code: "textBytes", constraint: maximum, path: [] } : null;
}
