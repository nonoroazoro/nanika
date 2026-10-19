import type { ContextMenuEntry } from "./ContextMenuEntry";

export interface ContextMenuSection
{
    title?: string;
    items: ContextMenuEntry[];
}
