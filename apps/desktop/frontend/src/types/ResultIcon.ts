import type { ActionIcon } from "./ActionIcon";

export type ResultIcon =
    | { kind: "image"; url: string; }
    | { kind: "symbol"; name: ActionIcon; };
