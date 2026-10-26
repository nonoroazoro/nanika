import { formatBytes } from "./bytes";
import { validOutput } from "./output";
import { formatText } from "./text";

import type { PresentationField } from "./PresentationField";
import type { ResourceSet } from "./ResourceSet";
import type { SupportedLocale } from "../generated/SupportedLocale";
import type { TargetPlatform } from "../generated/TargetPlatform";
import type { ViewMetadataValue } from "../generated/ViewMetadataValue";

/**
 * Immutable translations for one accepted publication or open surface.
 * Preparation checks both locales; rendering and warm switching only select stored strings.
 */
export class TextPresentation
{
    readonly resources: ResourceSet | undefined;
    private readonly _values = new Map<string, Record<SupportedLocale, string>>();

    constructor(resources: ResourceSet | undefined, platform: TargetPlatform, fields: readonly PresentationField[])
    {
        this.resources = resources;
        for (const { value, maximum, allowEmpty = false } of fields)
        {
            const values = { en: "", "zh-Hans": "" };
            for (const locale of ["en", "zh-Hans"] as const)
            {
                let text: string | undefined;
                if (typeof value === "string")
                {
                    text = value;
                }
                else if ("bytes" in value)
                {
                    text = formatBytes(value.bytes, locale);
                }
                else
                {
                    text = formatText(value, resources, locale, platform, maximum, allowEmpty);
                }
                if (text === undefined || !validOutput(text, maximum, allowEmpty))
                {
                    throw new Error("Presentation could not be rendered within its limits.");
                }
                values[locale] = text;
            }
            this._values.set(JSON.stringify(value), values);
        }
    }

    /**
     * Select prepared text without formatting or accessing the host.
     * @param value Original literal, package message reference or typed metadata
     * @param locale Authoritative presentation locale
     */
    text(value: ViewMetadataValue, locale: SupportedLocale): string
    {
        if (typeof value === "string")
        {
            return value;
        }
        const prepared = this._values.get(JSON.stringify(value));
        if (!prepared)
        {
            throw new Error("The text was not prepared for this publication.");
        }
        return prepared[locale];
    }

    /**
     * Check whether a value belongs to this prepared publication.
     * @param value Original literal, package message reference or typed metadata
     */
    has(value: ViewMetadataValue): boolean
    {
        return this._values.has(JSON.stringify(value));
    }
}
