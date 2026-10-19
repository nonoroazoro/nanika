import type { SyntaxTheme } from "./SyntaxTheme";

/**
 * Authority and immutable revision are supplied by the owning document service.
 */
export interface SyntaxIdentity
{
    readonly scope: string;
    readonly id: string;
    readonly revision: string;
    readonly theme: SyntaxTheme;
}
