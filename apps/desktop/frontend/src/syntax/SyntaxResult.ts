import type { SyntaxLanguage } from "./SyntaxLanguage";

export interface SyntaxResult
{
    language: SyntaxLanguage;

    /**
     * Ordered, non-overlapping document-relative UTF-16 start/end/palette triples.
     */
    ranges: Uint32Array;
}
