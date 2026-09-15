export type ViewEvent =
    | { action_id: string; item_id: string | null; kind: "actionInvoked"; }
    | { cursor: string; kind: "loadMore"; }
    | { filter_id: string; kind: "filterChanged"; value: string; }
    | { item_id: string | null; kind: "selectionChanged"; }
    | { kind: "resumed"; }
    | { kind: "searchChanged"; text: string; };
