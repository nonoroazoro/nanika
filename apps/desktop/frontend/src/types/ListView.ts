import type { DetailView, ViewFilter, ViewPagination, ViewSection } from "./index";
export interface ListView
{
    title: string;
    search_placeholder: string;
    search_text: string;
    layout: "plain" | "split";
    sections: ViewSection[];
    selected_item_id: string | null;
    detail: DetailView | null;
    filter: ViewFilter | null;
    pagination: ViewPagination | null;
}
