import type { StateStack } from "vscode-textmate";

import type { SyntaxLanguage } from "./SyntaxLanguage";
import type { SyntaxTheme } from "./SyntaxTheme";

/**
 * Only the unfinished line and its incoming grammar state survive a batch.
 */
export interface SyntaxCursor
{
    offset: number;
    lineStart: number;
    pending: string;
    skipping: boolean;
    state: StateStack;
    language: SyntaxLanguage;
    theme: SyntaxTheme;
}
