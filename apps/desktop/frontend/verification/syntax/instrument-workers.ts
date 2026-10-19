let trace: ((stage: string) => void) | null = null;

/**
 * Record message boundaries only during a latency probe, never retain source payloads.
 *
 * @param listener Active probe callback, or null when timing is disabled
 */
export function traceWorkers(listener: ((stage: string) => void) | null): void
{
    trace = listener;
}

/**
 * Install before page scripts. Count owned Workers without retaining Worker objects or source.
 */
export function instrumentWorkers(): void
{
    const metrics = { created: 0, terminated: 0, received: 0, pending: 0 };
    Object.assign(window, { syntaxWorkerMetrics: metrics });
    const NativeWorker = window.Worker;
    window.Worker = class extends NativeWorker
    {
        private _terminated = false;
        private readonly _pending = new Set<number>();

        constructor(url: string | URL, options?: WorkerOptions)
        {
            super(url, options);
            metrics.created++;
            trace?.("worker-created");
            this.addEventListener("message", (event: MessageEvent<{ id?: number; kind?: string; }>) =>
            {
                metrics.received++;
                if (
                    event.data.kind !== "tokenizing" && event.data.id !== undefined
                    && this._pending.delete(event.data.id)
                )
                {
                    metrics.pending--;
                }
                if (event.data.kind === "ready" || event.data.kind === "result" || event.data.kind === "detected")
                {
                    trace?.(`worker-${event.data.kind}`);
                }
            });
        }

        override postMessage(message: unknown, options?: StructuredSerializeOptions | Transferable[]): void
        {
            if (
                typeof message === "object" && message !== null && "kind" in message
                && (message.kind === "tokenize" || message.kind === "detect") && "id" in message
                && typeof message.id === "number"
            )
            {
                this._pending.add(message.id);
                metrics.pending++;
            }
            if (typeof message === "object" && message !== null && "kind" in message && message.kind === "tokenize")
            {
                trace?.("worker-send");
            }
            if (typeof message === "object" && message !== null && "kind" in message && message.kind === "detect")
            {
                trace?.("worker-detect");
            }
            if (typeof message === "object" && message !== null && "kind" in message && message.kind === "cancel")
            {
                trace?.("worker-cancel");
            }
            if (Array.isArray(options))
            {
                super.postMessage(message, options);
            }
            else
            {
                super.postMessage(message, options);
            }
        }

        override terminate(): void
        {
            if (!this._terminated)
            {
                this._terminated = true;
                metrics.terminated++;
                metrics.pending -= this._pending.size;
                this._pending.clear();
            }
            super.terminate();
        }
    };
}
