import type { ViewItem } from "./index";
export interface ViewSection
{
    id: string;
    title: string | null;
    items: ViewItem[];
}
