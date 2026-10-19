import type { SyntaxLanguage } from "./SyntaxLanguage";
import type { SyntaxResult } from "./SyntaxResult";

export type SyntaxWorkerResponse =
    | { id: number; kind: "cancelled"; }
    | { id: number; kind: "detected"; language: SyntaxLanguage | null; }
    | { id: number; kind: "result"; result: SyntaxResult; }
    | { id: number; kind: "tokenizing"; }
    | { id: number | null; kind: "error"; }
    | { kind: "ready"; };
