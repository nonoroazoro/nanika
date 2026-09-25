import type { ExtensionViewDocument } from "./index";
export interface ExtensionViewSnapshot
{
    routeId: number;
    extensionId: string;
    instanceId: number;
    generation: number;
    viewId: string;
    revision: number;
    view: ExtensionViewDocument;
}
