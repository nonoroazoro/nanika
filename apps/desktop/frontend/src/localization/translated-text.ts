import { validOutput } from "./output";
import { localeMatching } from "../generated/locale-matching";

const LANGUAGE_TAG =
    /^[a-z]{2,3}(?:-[A-Z][a-z]{3})?(?:-(?:[A-Z]{2}|[0-9]{3}))?(?:-(?:[a-z0-9]{5,8}|[0-9][a-z0-9]{3}))*$/;

/**
 * Validate all supplied translations before selecting one; matching language variants precede English and the source fallback.
 * @param value Domain-owned translated text
 * @param locale Authoritative UI locale
 * @param maximum Destination Unicode scalar limit
 * @param allowEmpty Whether the destination permits empty text
 */
export function formatTranslatedText(value: unknown, locale: string, maximum: number, allowEmpty = false): string
{
    if (!value || typeof value !== "object" || Array.isArray(value))
    {
        throw new Error("Invalid translated text.");
    }
    const text = value as Record<string, unknown>;
    if (
        Object.keys(text).some(key => key !== "fallback" && key !== "translations")
        || typeof text.fallback !== "string" || !text.translations || typeof text.translations !== "object"
        || Array.isArray(text.translations)
    )
    {
        throw new Error("Invalid translated text.");
    }
    const entries = Object.entries(text.translations);
    const encoder = new TextEncoder();
    let bytes = encoder.encode(text.fallback).byteLength;
    if (entries.length === 0 || !validOutput(text.fallback, maximum, allowEmpty))
    {
        throw new Error("Translated text exceeds its limits.");
    }
    for (const [language, translated] of entries)
    {
        if (
            language.length > 64
            || !LANGUAGE_TAG.test(language)
            || typeof translated !== "string" || !validOutput(translated, maximum, allowEmpty)
        )
        {
            throw new Error("Invalid translated text entry.");
        }
        // Rust owns canonical protocol keys. WebView ICU versions may normalize them differently;
        // check syntax here without rejecting host-admitted names or rewriting their lookup keys.
        Intl.getCanonicalLocales(language);
        bytes += encoder.encode(language).byteLength + encoder.encode(translated).byteLength;
    }
    if (bytes > 65_536)
    {
        throw new Error("Translated text exceeds 65536 bytes.");
    }
    const translations = text.translations as Record<string, string>;
    return _match(translations, locale) ?? _match(translations, "en") ?? text.fallback;
}

function _match(translations: Record<string, string>, locale: string): string | undefined
{
    if (Object.hasOwn(translations, locale))
    {
        return translations[locale];
    }
    const [language, script, region] = _parts(locale);
    let best: { score: number; tag: string; value: string; } | undefined;
    for (const [tag, value] of Object.entries(translations))
    {
        // Rust has canonicalized source keys. Infer script/region only after matching that language.
        if (tag.split("-", 1)[0] !== language)
        {
            continue;
        }
        const [, availableScript, availableRegion] = _parts(tag);
        const score = (2 * Number(availableScript === script))
            + Number(availableRegion === region);
        if (!best || score > best.score || (score === best.score && tag < best.tag))
        {
            best = { score, tag, value };
        }
    }
    return best?.value;
}

function _parts(tag: string): [string, string | undefined, string | undefined]
{
    const parts = /^([a-z]{2,3})(?:-([A-Z][a-z]{3}))?(?:-([A-Z]{2}|[0-9]{3}))?/.exec(tag);
    if (!parts?.[1])
    {
        throw new Error("Invalid language tag.");
    }
    const language = parts[1];
    const [, , rawScript, rawRegion] = parts;
    const script = rawScript === "Zzzz" ? undefined : rawScript;
    const region = rawRegion === "ZZ" ? undefined : rawRegion;
    const data = localeMatching[language];
    return [
        language,
        script ?? data?.regions[region ?? ""] ?? data?.script,
        region ?? data?.scripts[script ?? ""] ?? data?.region
    ];
}
