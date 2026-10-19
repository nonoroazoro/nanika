import type { ContextMenuSection } from "./ContextMenuSection";
import type { Action } from "../generated/Action";
import type { ConfigurationMenu } from "../generated/ConfigurationMenu";
import type { ConfigurationMenuRequest } from "../generated/ConfigurationMenuRequest";
import type { ContextMenuRequest } from "../generated/ContextMenuRequest";

export interface ContextMenuPresentation
{
    settingsExtensionId?: string;
    configurationMenu?: { request: ConfigurationMenuRequest; snapshot: ConfigurationMenu; };
    sections?: ContextMenuSection[];
    extensionIconUrl?: string;
    request: ContextMenuRequest | null;
    heading?: string;
    shortcuts?: Record<string, string[]>;
    actions: Action[];
    position: [number, number] | null;
    anchor?: HTMLElement;
}
