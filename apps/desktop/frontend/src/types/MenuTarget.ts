export type MenuTarget =
    | { entryId: string; extensionId: string; kind: "search"; requestId: number; resultRevision: number; }
    | { itemId: string | null; kind: "view"; revision: number; routeId: number; };
