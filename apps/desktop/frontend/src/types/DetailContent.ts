import type { DetailImage } from "./index";

export type DetailContent =
    | { alternative_text: string; kind: "image"; source: DetailImage; }
    | { kind: "files"; names: string[]; }
    | { kind: "text"; value: string; };
