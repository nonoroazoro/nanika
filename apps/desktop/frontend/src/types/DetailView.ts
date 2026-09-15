import type { DetailContent, ViewAction, ViewMetadata } from "./index";
export interface DetailView
{
    title: string | null;
    content: DetailContent;
    metadata: ViewMetadata[];
    actions: ViewAction[];
}
