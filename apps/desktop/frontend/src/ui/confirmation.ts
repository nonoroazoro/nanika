import { readonly, writable } from "svelte/store";

import type { ConfirmationRequest } from "../types/ConfirmationRequest";

const request = writable<ConfirmationRequest | null>(null);
let resolve: ((confirmed: boolean) => void) | null = null;
export const confirmation = readonly(request);

/**
 * Request the shared modal. Concurrent activations never replace a reviewed target.
 * Callers retain and revalidate their invocation authority after this resolves.
 * @param details The question, consequences, action label and optional icon to display.
 */
export async function confirmAction(details: ConfirmationRequest): Promise<boolean>
{
    if (resolve)
    {
        return Promise.resolve(false);
    }
    return new Promise<boolean>(complete =>
    {
        resolve = complete;
        request.set(details);
    });
}

/**
 * Keep application-level keyboard shortcuts outside the modal's focus scope.
 */
export function confirmationPending(): boolean
{
    return resolve !== null;
}

/**
 * Settle at most once, including cancellation on unmount or window deactivation.
 * @param confirmed Whether the user explicitly accepted the displayed operation.
 */
export function settleConfirmation(confirmed: boolean): void
{
    const complete = resolve;
    resolve = null;
    request.set(null);
    complete?.(confirmed);
}
