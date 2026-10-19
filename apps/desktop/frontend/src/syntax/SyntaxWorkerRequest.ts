import type { SyntaxContinuation } from "./SyntaxContinuation";
import type { SyntaxLanguage } from "./SyntaxLanguage";
import type { SyntaxTheme } from "./SyntaxTheme";

export type SyntaxWorkerRequest =
    | {
        continuation: SyntaxContinuation;
        id: number;
        kind: "tokenize";
        language: SyntaxLanguage;
        text: string;
        theme: SyntaxTheme;
    }
    | { id: number; kind: "cancel"; }
    | { id: number; kind: "detect"; text: string; }
    | { kind: "release"; stream: number; };
