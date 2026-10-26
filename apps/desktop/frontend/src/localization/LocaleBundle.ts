import { FluentBundle, FluentNumber, FluentResource } from "@fluent/bundle";

import type { FluentVariable } from "@fluent/bundle";

import { validOutput } from "./output";
import { formatTranslatedText } from "./translated-text";
import {
    MAX_LOCALE_BYTES,
    MAX_LOCALE_RESOURCES,
    MAX_MESSAGE_ARGUMENT_BYTES,
    MAX_MESSAGE_ARGUMENTS,
    MAX_MESSAGE_ID_BYTES,
    MAX_RESOURCE_BYTES
} from "../generated/localization-limits";

import type { LocalizedText } from "../generated/LocalizedText";
import type { MessageContract } from "../generated/MessageContract";
import type { SupportedLocale } from "../generated/SupportedLocale";

/**
 * Immutable presentation bundle built only from resources and contracts validated by Rust.
 * The caller binds this object to its authorized package identity and lifetime.
 */
export class LocaleBundle
{
    private readonly _bundle: FluentBundle;
    private readonly _locale: SupportedLocale;
    private readonly _contracts: ReadonlyMap<string, readonly string[]>;

    constructor(
        locale: SupportedLocale,
        sources: readonly string[],
        contracts: Readonly<Record<string, MessageContract>>
    )
    {
        this._locale = locale;
        const encoder = new TextEncoder();
        const bytes = sources.map(source => encoder.encode(source).byteLength);
        if (
            sources.length === 0 || sources.length > MAX_LOCALE_RESOURCES
            || bytes.some(size => size > MAX_RESOURCE_BYTES)
            || bytes.reduce((sum, size) => sum + size, 0) > MAX_LOCALE_BYTES
        )
        {
            throw new Error("Fluent resource budget exceeded.");
        }
        const bundle = new FluentBundle(locale);
        for (const source of sources)
        {
            if (bundle.addResource(new FluentResource(source)).length > 0)
            {
                throw new Error("Duplicate or invalid Fluent resources.");
            }
        }
        this._contracts = new Map(
            Object.entries(contracts).map(([id, contract]) =>
            {
                if (bundle.getMessage(id)?.value == null)
                {
                    throw new Error(`${id}: missing message value`);
                }
                return [id, [...contract.arguments].filter(name => name !== "platform").sort()];
            })
        );
        this._bundle = bundle;
    }

    /**
     * Test a validated message contract when selecting the package's fallback bundle.
     *
     * @param id Message identifier
     */
    hasMessage(id: string): boolean
    {
        return this._contracts.has(id);
    }

    /**
     * Resolve plain text without IPC, resource parsing, or mutation of the shared bundle.
     *
     * @param text Literal content or an owner-bound message reference
     * @param platform Host-supplied platform context
     * @param maximumChars Destination field's Unicode scalar limit
     * @param allowEmpty Whether the destination permits blank text
     */
    format(text: LocalizedText, platform: "macos" | "windows", maximumChars: number, allowEmpty = false): string
    {
        let result: string;
        if (typeof text === "string")
        {
            result = text;
        }
        else if ("translations" in text)
        {
            result = formatTranslatedText(text, this._locale, maximumChars, allowEmpty);
        }
        else
        {
            const expected = this._contracts.get(text.message);
            const entries = Object.entries(text.args ?? {});
            if (
                !_identifier(text.message) || !expected || entries.length > MAX_MESSAGE_ARGUMENTS
                || entries.length !== expected.length
                || entries.some(([name]) => !expected.includes(name) || name === "platform" || !_identifier(name))
            )
            {
                throw new Error(`${text.message}: invalid message or argument contract`);
            }
            const args: Record<string, FluentVariable> = Object.create(null) as Record<string, FluentVariable>;
            args.platform = platform;
            const encoder = new TextEncoder();
            for (const [name, value] of entries)
            {
                if (typeof value === "number" && Number.isSafeInteger(value))
                {
                    // Retain the number for plural selection; -0 has the same JSON integer meaning as 0.
                    args[name] = new FluentNumber(value === 0 ? 0 : value, {
                        useGrouping: false,
                        maximumFractionDigits: 0
                    });
                }
                else if (
                    typeof value === "string" && encoder.encode(value).byteLength <= MAX_MESSAGE_ARGUMENT_BYTES
                    && !value.includes("\0")
                )
                {
                    args[name] = value;
                }
                else
                {
                    throw new Error(`${text.message}: invalid argument ${name}`);
                }
            }
            const value = this._bundle.getMessage(text.message)?.value;
            if (value == null)
            {
                throw new Error(`${text.message}: missing message value`);
            }
            const errors: Error[] = [];
            result = this._bundle.formatPattern(value, args, errors);
            if (errors.length > 0)
            {
                throw new Error(`${text.message}: formatting failed`);
            }
        }
        if (!validOutput(result, maximumChars, allowEmpty))
        {
            throw new Error("Localized output is empty, oversized, or contains invalid control characters.");
        }
        return result;
    }
}

function _identifier(value: string): boolean
{
    return value.length <= MAX_MESSAGE_ID_BYTES && /^[a-z][a-z0-9_-]*$/iu.test(value);
}
