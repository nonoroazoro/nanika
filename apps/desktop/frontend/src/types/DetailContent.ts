import type { DetailImage, IconReference, ViewPagination } from "./index";

export type DetailContent =
    | { alternative_text: string; kind: "image"; source: DetailImage; }
    | { files: Array<{ icon: IconReference | null; name: string; path: string; }>; kind: "files"; }
    | { kind: "text"; pagination: ViewPagination | null; value: string; };
