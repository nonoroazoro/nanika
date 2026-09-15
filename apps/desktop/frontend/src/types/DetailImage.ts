export type DetailImage =
    | { kind: "dataUrl"; value: string; }
    | { kind: "resource"; path: string; };
