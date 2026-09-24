export type MenuTarget =
    | { entryId: string; extensionId: string; kind: "search"; requestId: number; revision: number; }
    | { itemId: string | null; kind: "view"; revision: number; routeId: number; };
