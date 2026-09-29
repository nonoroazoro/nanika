import type { ViewItem } from "./index";
export interface ViewSection
{
    id: string;
    title: string | null;
    offset: number;
    total: number;
    items: ViewItem[];
}
