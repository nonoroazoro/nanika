import type { ContextMenuEntry } from "./ContextMenuEntry";

export interface ContextMenuSection
{
    id?: string;
    title?: string;
    items: ContextMenuEntry[];
}
