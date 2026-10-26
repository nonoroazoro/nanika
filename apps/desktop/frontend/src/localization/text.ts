import { validOutput } from "./output";
import { formatTranslatedText } from "./translated-text";

import type { ResourceSet } from "./ResourceSet";
import type { LocalizedText } from "../generated/LocalizedText";
import type { SupportedLocale } from "../generated/SupportedLocale";
import type { TargetPlatform } from "../generated/TargetPlatform";

/**
 * Resolve domain text independently of package-resource availability.
 * @param text Original presentation source
 * @param resources Resources owned by the publishing extension
 * @param locale Authoritative UI locale
 * @param platform Host platform
 * @param maximum Destination Unicode scalar limit
 * @param allowEmpty Whether the destination permits empty text
 */
export function formatText(
    text: LocalizedText,
    resources: ResourceSet | undefined,
    locale: SupportedLocale,
    platform: TargetPlatform,
    maximum: number,
    allowEmpty = false
): string
{
    if (typeof text === "string")
    {
        if (!validOutput(text, maximum, allowEmpty))
        {
            throw new Error("Text exceeds its presentation limits.");
        }
        return text;
    }
    if ("translations" in text)
    {
        return formatTranslatedText(text, locale, maximum, allowEmpty);
    }
    if (!resources)
    {
        throw new Error("Message reference requires localization resources.");
    }
    return resources.format(locale, text, platform, maximum, allowEmpty);
}
