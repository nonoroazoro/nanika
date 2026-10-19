/**
 * A caller-owned stream. Offsets must be contiguous within one worker lifetime.
 */
export interface SyntaxContinuation
{
    readonly stream: number;
    readonly offset: number;
    readonly final: boolean;
}
