import type { ExtensionViewSnapshot } from "./index";
export interface NavigationSnapshot
{
    revision: number;
    current: ExtensionViewSnapshot | null;
    busy: boolean;
    error: string | null;
    dismissCount: number;
}
