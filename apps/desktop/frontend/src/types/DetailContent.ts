import type { DetailImage, IconReference } from "./index";

export type DetailContent =
    | { alternative_text: string; kind: "image"; source: DetailImage; }
    | { chunk_index: number; kind: "text"; text_id: string; total_chunks: number; value: string; }
    | { files: Array<{ icon: IconReference | null; name: string; path: string; }>; kind: "files"; };
