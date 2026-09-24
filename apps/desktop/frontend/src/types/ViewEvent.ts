import type { ActionInvocation } from "./ActionInvocation";

export type ViewEvent =
    | { action_id: string; invocation: ActionInvocation; item_id: string | null; kind: "actionInvoked"; }
    | { cursor: string; kind: "loadMore"; }
    | { filter_id: string; kind: "filterChanged"; value: string; }
    | { item_id: string | null; kind: "selectionChanged"; }
    | { kind: "resumed"; }
    | { kind: "searchChanged"; text: string; };
