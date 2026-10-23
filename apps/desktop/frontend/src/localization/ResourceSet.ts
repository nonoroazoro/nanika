import { LocaleBundle } from "./LocaleBundle";

import type { LocalizedText } from "../generated/LocalizedText";
import type { PreparedResources } from "../generated/PreparedResources";
import type { SupportedLocale } from "../generated/SupportedLocale";

/**
 * Retained immutable resources. A caller replaces this object only after every bundle is ready.
 */
export class ResourceSet
{
    readonly identity: string;
    private readonly _default: LocaleBundle;
    private readonly _bundles: ReadonlyMap<SupportedLocale, LocaleBundle>;

    constructor(prepared: PreparedResources)
    {
        const bundles = new Map<SupportedLocale, LocaleBundle>();
        for (const locale of ["en", "zh-Hans"] as const)
        {
            const resource = prepared.locales[locale];
            if (resource)
            {
                bundles.set(locale, new LocaleBundle(locale, resource.sources, resource.contracts));
            }
        }
        const fallback = bundles.get(prepared.defaultLocale);
        if (!fallback)
        {
            throw new Error("Default locale has no prepared bundle.");
        }
        this.identity = prepared.identity;
        this._default = fallback;
        this._bundles = bundles;
    }

    /**
     * Fall back per message within its owner's complete default bundle, including its terms.
     *
     * @param locale Authoritative host locale
     * @param text Owner-bound message or literal content
     * @param platform Host platform
     * @param maximumChars Destination field limit
     * @param allowEmpty Whether the destination permits blank text
     */
    format(
        locale: SupportedLocale,
        text: LocalizedText,
        platform: "macos" | "windows",
        maximumChars: number,
        allowEmpty = false
    ): string
    {
        const current = this._bundles.get(locale);
        const bundle = current && (typeof text === "string" || current.hasMessage(text.message))
            ? current
            : this._default;
        return bundle.format(text, platform, maximumChars, allowEmpty);
    }
}
