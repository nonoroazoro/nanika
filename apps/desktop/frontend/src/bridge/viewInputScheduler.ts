import type { NavigationSnapshot } from "../types/NavigationSnapshot.ts";
import type { ViewEvent } from "../types/ViewEvent.ts";
import type { ViewEventReceipt } from "../types/ViewEventReceipt.ts";

// RPC completion and Channel delivery are independent. Follow-up input needs
// both, including the exact completed navigation revision rather than any update.
export function viewInputScheduler()
{
    let navigation: NavigationSnapshot = { revision: 0, current: null, busy: false, error: null, dismissCount: 0 };
    let pending = 0;
    let blocking = 0;
    let requiredRevision = 0;
    let blockingRevision = 0;
    let query: string | null = null;
    let resumeRequested = false;
    let inputError: string | null = null;

    return {
        get busy(): boolean
        {
            return blocking > 0 || navigation.revision < blockingRevision || (pending === 0 && navigation.busy);
        },
        get inputError(): string | null
        {
            return inputError;
        },
        update(next: NavigationSnapshot): void
        {
            if (next.revision <= navigation.revision)
            {
                return;
            }
            if (next.current?.routeId !== navigation.current?.routeId)
            {
                query = null;
                resumeRequested = false;
                inputError = null;
            }
            navigation = next;
        },
        query(text: string): void
        {
            query = text;
            // Match the Rust protocol's Unicode scalar count, not grapheme count.
            inputError = Array.from(text).length > 4096
                ? "View search supports up to 4096 characters. Edit the input to continue."
                : null;
        },
        resume(): void
        {
            resumeRequested = true;
        },
        begin(event: ViewEvent | null): boolean
        {
            const isBlocking = event?.kind !== "selectionChanged" && event?.kind !== "resumed";
            pending++;
            if (isBlocking)
            {
                blocking++;
            }
            return isBlocking;
        },
        complete(isBlocking: boolean, receipt: ViewEventReceipt | null): void
        {
            pending--;
            if (isBlocking)
            {
                blocking--;
            }
            if (receipt)
            {
                requiredRevision = Math.max(requiredRevision, receipt.navigationRevision);
                if (isBlocking)
                {
                    blockingRevision = Math.max(blockingRevision, receipt.navigationRevision);
                }
            }
        },
        takeNext(): ViewEvent | null
        {
            const current = navigation.current;
            if (pending > 0 || navigation.busy || navigation.revision < requiredRevision || !current)
            {
                return null;
            }
            if (query !== null && !inputError && current.view.kind === "list")
            {
                const text = query;
                // Consume submitted intent. Failure never automatically retries it;
                // a newer query remains independently eligible after completion.
                query = null;
                if (text !== current.view.list.search_text)
                {
                    return { kind: "searchChanged", text };
                }
            }
            if (resumeRequested)
            {
                resumeRequested = false;
                return { kind: "resumed" };
            }
            return null;
        }
    };
}
