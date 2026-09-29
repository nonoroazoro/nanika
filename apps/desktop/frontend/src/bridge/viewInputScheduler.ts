import type { NavigationSnapshot } from "../types/NavigationSnapshot.ts";
import type { ViewEvent } from "../types/ViewEvent.ts";
import type { ViewEventReceipt } from "../types/ViewEventReceipt.ts";

// RPC completion and Channel delivery are independent. Follow-up input needs
// both, including the exact completed navigation revision rather than any update.
export function viewInputScheduler()
{
    let navigation: NavigationSnapshot = {
        revision: 0,
        current: null,
        busy: false,
        error: null,
        dismissCount: 0
    };
    let pending = 0;
    let blocking = 0;
    let requiredRevision = 0;
    let blockingRevision = 0;
    let query: Extract<ViewEvent, { kind: "searchChanged"; }> | null = null;
    let draft: string | null = null;
    let resumeRequested = false;
    let inputError: string | null = null;

    return {
        get busy(): boolean
        {
            // A receipt can arrive before the Channel clears the operation's old busy snapshot.
            // Only fresh authority may introduce host-owned blocking after our requests finish.
            return blocking > 0 || navigation.revision < blockingRevision
                || (pending === 0 && navigation.revision >= requiredRevision && navigation.busy);
        },
        get canLoadMore(): boolean
        {
            return pending === 0 && !navigation.busy && navigation.revision >= requiredRevision
                && query === null && draft === null && !resumeRequested;
        },
        get queryText(): string
        {
            const current = navigation.current;
            const authoritative = current?.view.kind === "list" ? current.view.list.search_text : "";
            // A draft owns the input until both transports settle its submission.
            // A newer queued intent (including invalid input) outlives that submission.
            return draft ?? authoritative;
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
                draft = null;
                resumeRequested = false;
                inputError = null;
            }
            navigation = next;
            _settleDraft();
        },
        query(text: string, minimumItems: number): void
        {
            query = { kind: "searchChanged", text, minimum_items: minimumItems };
            draft = text;
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
            const isBlocking = event?.kind !== "selectionChanged" && event?.kind !== "resumed"
                && event?.kind !== "listRangeChanged" && event?.kind !== "textChunkRequested";
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
            _settleDraft();
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
                const event = query;
                // Consume this intent without retry; newer queries remain eligible after completion.
                query = null;
                if (event.text !== current.view.list.search_text)
                {
                    return event;
                }
                _settleDraft();
            }
            if (resumeRequested)
            {
                resumeRequested = false;
                return { kind: "resumed" };
            }
            return null;
        }
    };

    function _settleDraft(): void
    {
        if (query === null && pending === 0 && !navigation.busy && navigation.revision >= requiredRevision)
        {
            draft = null;
        }
    }
}
