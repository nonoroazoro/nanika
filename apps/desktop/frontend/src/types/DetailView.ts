import type { Action, DetailContent, ViewMetadata } from "./index";
export interface DetailView
{
    title: string | null;
    content: DetailContent;
    metadata: ViewMetadata[];
    actions: Action[];
}
