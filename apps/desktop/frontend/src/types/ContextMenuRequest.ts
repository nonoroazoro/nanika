import type { MenuTarget } from "./MenuTarget";

export interface ContextMenuRequest
{
    sessionId: number;
    target: MenuTarget;
}
