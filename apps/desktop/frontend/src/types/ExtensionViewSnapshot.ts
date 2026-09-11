import type { ExtensionViewDocument } from "./index";
export interface ExtensionViewSnapshot
{
    routeId: number;
    extensionId: string;
    generation: number;
    viewId: string;
    revision: number;
    view: ExtensionViewDocument;
}
