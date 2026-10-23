import { ResourceSet } from "./ResourceSet";

import type { LanguagePreference } from "../generated/LanguagePreference";
import type { LocalizationSnapshot } from "../generated/LocalizationSnapshot";
import type { LocalizedText } from "../generated/LocalizedText";
import type { PreparedResources } from "../generated/PreparedResources";
import type { SupportedLocale } from "../generated/SupportedLocale";
import type { TargetPlatform } from "../generated/TargetPlatform";

/**
 * One WebView's presentation state. Language changes never own business state or resource lifetime.
 */
export class LocalizationState
{
    private _locale = $state<SupportedLocale>("en");
    private _preference = $state<LanguagePreference>("system");
    private _platform = $state<TargetPlatform>("macos");
    private _sessionId: number | null = null;
    private _revision = 0;
    private _pending: { sessionId: number; snapshot: LocalizationSnapshot; } | null = null;
    private readonly _resources: ResourceSet;

    constructor(resources: PreparedResources)
    {
        this._resources = new ResourceSet(resources);
    }

    get locale(): SupportedLocale
    {
        return this._locale;
    }

    get preference(): LanguagePreference
    {
        return this._preference;
    }

    /**
     * Establish authority from the initial reply, then apply any newer channel snapshot.
     *
     * @param sessionId The current window session
     * @param snapshot Initial host state
     * @param platform Host platform, independent of the language preference
     */
    initialize(sessionId: number, snapshot: LocalizationSnapshot, platform: TargetPlatform): void
    {
        if (this._sessionId !== sessionId)
        {
            this._sessionId = sessionId;
            this._revision = 0;
        }
        this._platform = platform;
        this.accept(sessionId, snapshot);
        const pending = this._pending;
        this._pending = null;
        if (pending)
        {
            this.accept(pending.sessionId, pending.snapshot);
        }
    }

    /**
     * Accept presentation independently of query and view revisions.
     *
     * @param sessionId Session that owns the event
     * @param snapshot Authoritative host state
     */
    accept(sessionId: number, snapshot: LocalizationSnapshot): boolean
    {
        if (this._sessionId === null)
        {
            if (
                this._pending?.sessionId !== sessionId
                || snapshot.revision > this._pending.snapshot.revision
            )
            {
                this._pending = { sessionId, snapshot };
            }
            return false;
        }
        if (sessionId !== this._sessionId || snapshot.revision <= this._revision)
        {
            return false;
        }
        this._revision = snapshot.revision;
        this._preference = snapshot.preference;
        // Separate reactive fields avoid reformatting when only Follow System changes.
        this._locale = snapshot.locale;
        return true;
    }

    /**
     * Resolve presentation from explicitly owned, retained resources.
     *
     * @param text Message reference or literal domain content
     * @param resources Package resources, or the host bundle when omitted
     * @param locale Locale prepared for a retained surface, defaulting to the active locale
     */
    text(text: LocalizedText, resources: ResourceSet = this._resources, locale: SupportedLocale = this._locale): string
    {
        return resources.format(locale, text, this._platform, 2048, true);
    }
}
