import type { TextPresentation } from "../localization/TextPresentation";

/** Prepared text and resource authority retained by the current result window. */
export interface RootPresentation
{
    sessionId: number;
    instanceId: number;
    presentation: TextPresentation;
}
