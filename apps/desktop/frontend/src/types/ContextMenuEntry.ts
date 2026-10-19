import type { Action } from "../generated/Action";

export interface ContextMenuEntry extends Action
{
    checked?: boolean;
}
