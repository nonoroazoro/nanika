import type { Action, ViewItemIcon } from "./index";
export interface ViewItem
{
    id: string;
    title: string;
    subtitle: string | null;
    icon: ViewItemIcon | null;
    actions: Action[];
}
