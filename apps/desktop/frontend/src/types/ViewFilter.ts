import type { ViewFilterOption } from "./index";
export interface ViewFilter
{
    id: string;
    selected_value: string;
    options: ViewFilterOption[];
}
