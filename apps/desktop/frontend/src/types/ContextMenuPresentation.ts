import type { Action } from "../generated/Action";
import type { ContextMenuRequest } from "../generated/ContextMenuRequest";

export interface ContextMenuPresentation
{
    request: ContextMenuRequest | null;
    heading?: string;
    shortcuts?: Record<string, string[]>;
    actions: Action[];
    position: [number, number] | null;
    anchor?: HTMLElement;
}
