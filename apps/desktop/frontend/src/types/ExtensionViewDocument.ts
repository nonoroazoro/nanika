import type { DetailView, ListView } from "./index";
export type ExtensionViewDocument = { detail: DetailView; kind: "detail"; } | { kind: "list"; list: ListView; };
