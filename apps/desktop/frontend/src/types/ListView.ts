import type { DetailView, ViewFilter, ViewSection } from "./index";
import type { ListSelection } from "./ListSelection";
export interface ListView
{
    title: string;
    search_placeholder: string;
    search_text: string;
    empty_title: string;
    empty_description: string;
    layout: "plain" | "split";
    sections: ViewSection[];
    collection_id: string;
    selection: ListSelection | null;
    detail: DetailView | null;
    filter: ViewFilter | null;
}
