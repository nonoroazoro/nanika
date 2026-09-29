export type SettingsSearchTarget =
    | { key: string; kind: "field" | "section"; }
    | { kind: "enabled" | "page"; };
