import type { DetailImage, IconReference } from "./index";

export type DetailContent =
    | { alternative_text: string; kind: "image"; source: DetailImage; }
    | { files: Array<{ icon: IconReference | null; name: string; path: string; }>; kind: "files"; }
    | { kind: "text"; value: string; };
