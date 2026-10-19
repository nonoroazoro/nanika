import type { SyntaxIdentity } from "./SyntaxIdentity";
import type { SyntaxLanguage } from "./SyntaxLanguage";
import type { SyntaxResult } from "./SyntaxResult";
import type { SyntaxWorkerRequest } from "./SyntaxWorkerRequest";

export type SyntaxJob = {
    identity: SyntaxIdentity;
    key: string;
    listeners: Set<(result: SyntaxLanguage | SyntaxResult | null) => void>;
} & Extract<SyntaxWorkerRequest, { kind: "detect" | "tokenize"; }>;
