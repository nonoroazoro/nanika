export interface StyleMotionOptions
{
    properties: string[];

    /**
     * Initial visual state for newly mounted content; CSS owns subsequent states.
     */
    enterFrom?: Record<string, string>;
    duration?: ((element: Element) => string) | string;
    ease?: string;
    scope?: string;
    requireFocus?: boolean;
}
