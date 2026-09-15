import type { ViewAction, ViewItemIcon } from "./index";
export interface ViewItem
{
    id: string;
    title: string;
    subtitle: string | null;
    icon: ViewItemIcon | null;
    actions: ViewAction[];
}
