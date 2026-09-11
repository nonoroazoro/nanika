import type { ViewAction, ViewMetadata } from "./index";
export interface DetailView
{
    title: string | null;
    body: string;
    image_data_url: string | null;
    metadata: ViewMetadata[];
    actions: ViewAction[];
}
