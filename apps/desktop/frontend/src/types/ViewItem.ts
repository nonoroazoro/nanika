import type { ViewAction } from "./index";
export interface ViewItem
{
    id: string;
    title: string;
    subtitle: string | null;
    actions: ViewAction[];
}
