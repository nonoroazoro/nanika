import type { Action } from "./Action";
import type { ContextMenuRequest } from "./ContextMenuRequest";

export interface ContextMenuPresentation
{
    request: ContextMenuRequest | null;
    heading?: string;
    shortcuts?: Record<string, string[]>;
    actions: Action[];
    position: [number, number] | null;
}
