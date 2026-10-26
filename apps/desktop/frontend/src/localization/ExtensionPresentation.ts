import { actionFields } from "./action-fields";
import { formatText } from "./text";
import { TextPresentation } from "./TextPresentation";

import type { ResourceSet } from "./ResourceSet";
import type { ConfigurationContribution } from "../generated/ConfigurationContribution";
import type { ExtensionResourceScope } from "../generated/ExtensionResourceScope";
import type { LocalizedText } from "../generated/LocalizedText";
import type { SupportedLocale } from "../generated/SupportedLocale";
import type { TargetPlatform } from "../generated/TargetPlatform";

/**
 * Prepared package presentation retained by its surface, independently of the reusable cache.
 * Both name translations are checked before publication, so a locale update cannot expose a partial label.
 */
export class ExtensionPresentation
{
    readonly resources: ResourceSet | undefined;
    readonly actions: TextPresentation;
    private readonly _key: string;
    private readonly _nameKey: string;
    private readonly _texts = new Map<string, Record<SupportedLocale, string>>();
    private readonly _platform: TargetPlatform;
    private readonly _names: Record<SupportedLocale, string>;

    constructor(
        extensionId: string,
        name: LocalizedText,
        scope: ExtensionResourceScope | undefined,
        resources: ResourceSet | undefined,
        platform: TargetPlatform,
        configuration?: ConfigurationContribution
    )
    {
        if (
            (scope && (scope.extensionId !== extensionId || resources?.identity !== scope.identity))
            || (!scope && resources)
        )
        {
            throw new Error("Extension presentation resources do not match their owner.");
        }
        this.resources = resources;
        this._nameKey = JSON.stringify(name);
        this._platform = platform;
        this._key = JSON.stringify([extensionId, name, scope, platform, configuration]);
        const names = { en: "", "zh-Hans": "" };
        for (const locale of ["en", "zh-Hans"] as const)
        {
            const value = formatText(name, resources, locale, platform, 128);
            if (!value?.trim() || /\p{Cc}/u.test(value) || new TextEncoder().encode(value).length > 128)
            {
                throw new Error("The extension name could not be rendered within its limits.");
            }
            names[locale] = value;
        }
        this._names = names;
        this.actions = new TextPresentation(
            resources,
            platform,
            actionFields(configuration?.actions.map(item => item.action) ?? [])
        );
        if (configuration)
        {
            this._prepare(configuration.title, 128);
            for (const property of Object.values(configuration.properties))
            {
                this._prepare(property.title, 128);
                if (property.group)
                {
                    this._prepare(property.group.title, 128);
                }
                if (property.description)
                {
                    this._prepare(property.description, 512);
                }
            }
            for (const item of configuration.actions)
            {
                this._prepare(item.description, 512);
                if (item.buttonLabel)
                {
                    this._prepare(item.buttonLabel, 128);
                }
            }
            for (const group of configuration.quickMenu)
            {
                this._prepare(group.title, 128);
            }
        }
    }

    /**
     * Return a prepared name without parsing or formatting on query updates.
     *
     * @param locale Authoritative presentation locale
     */
    name(locale: SupportedLocale): string
    {
        return this._names[locale];
    }

    /**
     * Resolve package text using the prepared name on repeated search renders.
     *
     * @param value Package-owned presentation source
     * @param locale Current presentation locale
     */
    text(value: LocalizedText, locale: SupportedLocale): string
    {
        if (typeof value === "string")
        {
            return value;
        }
        if (JSON.stringify(value) === this._nameKey)
        {
            return this.name(locale);
        }
        const prepared = this._texts.get(JSON.stringify(value));
        if (prepared)
        {
            return prepared[locale];
        }
        if (this.actions.has(value))
        {
            return this.actions.text(value, locale);
        }
        return formatText(value, this.resources, locale, this._platform, 2048, true);
    }

    /**
     * Reuse unchanged metadata while instance lifecycle and business state continue to update.
     *
     * @param extensionId Actual metadata owner
     * @param name Original literal or message reference
     * @param scope Host-bound content identity
     * @param platform Native presentation vocabulary
     * @param configuration Immutable declared configuration presentation
     */
    matches(
        extensionId: string,
        name: LocalizedText,
        scope: ExtensionResourceScope | undefined,
        platform: TargetPlatform,
        configuration?: ConfigurationContribution
    ): boolean
    {
        return this._key === JSON.stringify([extensionId, name, scope, platform, configuration]);
    }

    private _prepare(text: LocalizedText, maximum: number): void
    {
        const values = { en: "", "zh-Hans": "" };
        for (const locale of ["en", "zh-Hans"] as const)
        {
            const value = formatText(text, this.resources, locale, this._platform, maximum);
            if (!value?.trim() || new TextEncoder().encode(value).length > maximum)
            {
                throw new Error("The configuration text could not be rendered within its limits.");
            }
            values[locale] = value;
        }
        this._texts.set(JSON.stringify(text), values);
    }
}
