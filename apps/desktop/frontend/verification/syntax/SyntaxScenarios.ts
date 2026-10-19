export interface SyntaxScenarios
{
    benchmark(): Promise<unknown>;
    latency(): Promise<unknown>;
    prepareVisibility(): Promise<unknown>;
    verifyVisibility(): unknown;
    lifecycle(): Promise<unknown>;
}
