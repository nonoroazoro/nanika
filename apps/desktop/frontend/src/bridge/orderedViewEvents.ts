import type { ContextMenuRequest } from "../types/ContextMenuRequest.ts";
import type { ViewEventReceipt } from "../types/ViewEventReceipt.ts";
import type { ViewEventRequest } from "../types/ViewEventRequest.ts";

// A selection is replaceable only before dispatch and before a following action.
// The action keeps its captured item id and never overtakes earlier input.
export function orderedViewEvents(
    send: (request: ViewEventRequest) => Promise<ViewEventReceipt>,
    sendMenu: (request: ContextMenuRequest, actionId: string, confirmed: boolean) => Promise<ViewEventReceipt | null>
)
{
    interface Pending
    {
        request: ViewEventRequest | null;
        dispatch: () => Promise<ViewEventReceipt | null>;
        resolve: (receipt: ViewEventReceipt | null) => void;
        reject: (error: unknown) => void;
    }
    const pending: Pending[] = [];
    let running = false;

    return {
        event: async (request: ViewEventRequest) => _enqueue(request, async () => send(request)),
        // A menu is an action barrier. Its frozen snapshot is never coalesced or rebased.
        menu: async (request: ContextMenuRequest, actionId: string, confirmed: boolean) =>
            _enqueue(null, async () => sendMenu(request, actionId, confirmed))
    };

    async function _enqueue(
        request: ViewEventRequest | null,
        dispatch: () => Promise<ViewEventReceipt | null>
    ): Promise<ViewEventReceipt | null>
    {
        return new Promise((resolve, reject) =>
        {
            const previous = pending.at(-1);
            if (
                request?.operation.kind === "event" && request.operation.event.kind === "selectionChanged"
                && previous?.request?.operation.kind === "event"
                && previous.request.operation.event.kind === "selectionChanged"
                && previous.request.sessionId === request.sessionId && previous.request.routeId === request.routeId
            )
            {
                pending.pop();
                previous.resolve(null);
            }
            if (pending.length >= 16)
            {
                reject(new Error("View input is pending. Wait for the extension before continuing."));
                return;
            }
            pending.push({ request, dispatch, resolve, reject });
            void _drain();
        });
    }

    async function _drain(): Promise<void>
    {
        if (running)
        {
            return;
        }
        running = true;
        try
        {
            while (pending.length)
            {
                const next = pending.shift();
                if (!next)
                {
                    break;
                }
                try
                {
                    next.resolve(await next.dispatch());
                }
                catch (error)
                {
                    next.reject(error);
                }
            }
        }
        finally
        {
            running = false;
        }
    }
}
