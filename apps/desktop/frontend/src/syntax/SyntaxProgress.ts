import type { SyntaxResult } from "./SyntaxResult";

export interface SyntaxProgress
{
    /**
     * Consumed UTF-16 prefix, including any unfinished line held by the worker.
     */
    readonly processed: number;
    readonly result: SyntaxResult;
}
