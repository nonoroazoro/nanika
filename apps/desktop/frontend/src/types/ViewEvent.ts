import type { ActionInvocation } from "./ActionInvocation";

export type ViewEvent =
    | { action_id: string; invocation: ActionInvocation; item_id: string | null; kind: "actionInvoked"; }
    | { collection_id: string; count: number; kind: "listRangeChanged"; offset: number; }
    | { collection_id: string; index: number; kind: "selectionChanged"; }
    | { filter_id: string; kind: "filterChanged"; minimum_items: number; value: string; }
    | { index: number; kind: "textChunkRequested"; text_id: string; }
    | { kind: "resumed"; }
    | { kind: "searchChanged"; minimum_items: number; text: string; };
