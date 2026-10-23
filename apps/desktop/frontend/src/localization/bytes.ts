import type { SupportedLocale } from "../generated/SupportedLocale";

const UNITS = ["B", "KiB", "MiB", "GiB", "TiB", "PiB"] as const;
const formatters: Record<SupportedLocale, Intl.NumberFormat> = {
    en: new Intl.NumberFormat("en", { maximumFractionDigits: 1 }),
    "zh-Hans": new Intl.NumberFormat("zh-Hans", { maximumFractionDigits: 1 })
};

/**
 * Format an admitted measurement using IEC units and at most one decimal place.
 * This runs when a publication is prepared, never in row rendering or query processing.
 * @param bytes Nonnegative JSON-safe integer byte count
 * @param locale Authoritative presentation locale
 */
export function formatBytes(bytes: number, locale: SupportedLocale): string
{
    if (!Number.isSafeInteger(bytes) || bytes < 0)
    {
        throw new Error("Invalid metadata byte count.");
    }
    let unit = 0;
    let value = bytes;
    while (value >= 1024 && unit < UNITS.length - 1)
    {
        value /= 1024;
        unit++;
    }
    return `${formatters[locale].format(value)} ${UNITS[unit]}`;
}
