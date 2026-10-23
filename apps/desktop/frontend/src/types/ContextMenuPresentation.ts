import type { ContextMenuSection } from "./ContextMenuSection";
import type { Action } from "../generated/Action";
import type { ConfigurationMenu } from "../generated/ConfigurationMenu";
import type { ConfigurationMenuRequest } from "../generated/ConfigurationMenuRequest";
import type { ContextMenuRequest } from "../generated/ContextMenuRequest";
import type { SupportedLocale } from "../generated/SupportedLocale";
import type { ExtensionPresentation } from "../localization/ExtensionPresentation";
import type { TextPresentation } from "../localization/TextPresentation";

export interface ContextMenuPresentation
{
    presentation?: TextPresentation;
    locale?: SupportedLocale;
    settingsExtensionId?: string;
    configurationMenu?: { request: ConfigurationMenuRequest; snapshot: ConfigurationMenu; };
    sections?: ContextMenuSection[];
    localizedSections?: Record<SupportedLocale, ContextMenuSection[]>;
    extensionIconUrl?: string;
    request: ContextMenuRequest | null;
    heading?: string;
    headingPresentation?: ExtensionPresentation;
    shortcuts?: Record<string, string[]>;
    actions: Action[];
    position: [number, number] | null;
    anchor?: HTMLElement;
}
