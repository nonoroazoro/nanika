import { readonly, writable } from "svelte/store";

import type { ConfirmationRequest } from "../types/ConfirmationRequest";

const request = writable<ConfirmationRequest | null>(null);
let resolve: ((confirmed: boolean) => void) | null = null;
let decision: boolean | null = null;
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
 * Record the first decision and close the modal before releasing its caller.
 * The returned callback belongs to this request's completed exit only.
 * @param confirmed Whether the user accepted the displayed operation.
 */
export function closeConfirmation(confirmed: boolean): (() => void) | null
{
    if (!resolve || decision !== null)
    {
        return null;
    }
    decision = confirmed;
    const current = resolve;
    request.set(null);
    return () =>
    {
        if (resolve === current)
        {
            _settle(confirmed);
        }
    };
}

/**
 * Release an interrupted modal on unmount or window deactivation.
 * Preserve a recorded decision; an unanswered request is cancelled.
 */
export function interruptConfirmation(): void
{
    _settle(decision ?? false);
}

function _settle(confirmed: boolean): void
{
    const complete = resolve;
    resolve = null;
    decision = null;
    request.set(null);
    complete?.(confirmed);
}
