import type { ConfigurationProperty } from "../../src/generated/ConfigurationProperty";
import type { ConfigurationSchema } from "../../src/generated/ConfigurationSchema";

/**
 * Builds the complete schema shape serialized by Rust, including Serde defaults.
 *
 * @param overrides Fields relevant to the behavior under test
 */
export function configurationSchema(
    overrides: Partial<ConfigurationSchema> & Pick<ConfigurationSchema, "type">
): ConfigurationSchema
{
    return {
        allowUnlimited: false,
        format: null,
        minimum: null,
        maximum: null,
        multipleOf: null,
        maxLength: null,
        maxItems: null,
        items: null,
        properties: {},
        required: [],
        ...overrides
    };
}

/**
 * Adds property metadata to a complete {@link configurationSchema} fixture.
 *
 * @param overrides Required identity, value and persistence with test-specific fields
 */
export function configurationProperty(
    overrides:
        & Partial<ConfigurationProperty>
        & Pick<ConfigurationProperty, "default" | "persistence" | "title" | "type">
): ConfigurationProperty
{
    return {
        ...configurationSchema(overrides),
        description: null,
        platforms: [],
        order: 0,
        ...overrides
    };
}
