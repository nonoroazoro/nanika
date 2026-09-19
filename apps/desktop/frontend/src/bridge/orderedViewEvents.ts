import type { ViewEventReceipt } from "../types/ViewEventReceipt.ts";
import type { ViewEventRequest } from "../types/ViewEventRequest.ts";

// A selection is replaceable only before dispatch and before a following action.
// The action keeps its captured item id and never overtakes earlier input.
export function orderedViewEvents(send: (request: ViewEventRequest) => Promise<ViewEventReceipt>)
{
    interface Pending
    {
        request: ViewEventRequest;
        resolve: (receipt: ViewEventReceipt | null) => void;
        reject: (error: unknown) => void;
    }
    const pending: Pending[] = [];
    let running = false;

    async function drain(): Promise<void>
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
                    next.resolve(await send(next.request));
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

    return async (request: ViewEventRequest): Promise<ViewEventReceipt | null> =>
        new Promise((resolve, reject) =>
        {
            const previous = pending.at(-1);
            if (
                request.operation.kind === "event" && request.operation.event.kind === "selectionChanged"
                && previous?.request.operation.kind === "event"
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
            pending.push({ request, resolve, reject });
            void drain();
        });
}
