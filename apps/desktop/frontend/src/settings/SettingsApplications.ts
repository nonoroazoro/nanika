import type { SettingsWriteResult } from "./SettingsWriteResult";
import type { ConfigurationValue, SettingsApplicationUpdate } from "../types/Settings";

/**
 * Correlates submission with terminal Channel events, including completion before
 * the invoke reply. Each extension retains one result and one active waiter.
 */
export class SettingsApplications
{
    private readonly _latest = new Map<string, SettingsApplicationUpdate>();
    private readonly _pending = new Map<string, {
        requestId: number;
        resolve: (update: SettingsApplicationUpdate) => void;
    }>();

    record(update: SettingsApplicationUpdate): boolean
    {
        const pending = this._pending.get(update.extensionId);
        // Deliver the matching completion even if a later snapshot was recorded.
        if (pending?.requestId === update.requestId && update.result.status !== "running")
        {
            this._pending.delete(update.extensionId);
            pending.resolve(update);
        }
        const current = this._latest.get(update.extensionId);
        if (
            current && (current.requestId > update.requestId
                || (current.requestId === update.requestId
                    && (current.result.status !== "running"
                        || (current.result.progress !== null && update.result.status === "running"
                            && update.result.progress === null))))
        )
        {
            return false;
        }
        this._latest.set(update.extensionId, update);
        return true;
    }

    current(extensionId: string): SettingsApplicationUpdate | undefined
    {
        return this._latest.get(extensionId);
    }

    async completion(
        update: SettingsApplicationUpdate
    ): Promise<SettingsWriteResult<Record<string, ConfigurationValue>>>
    {
        this.record(update);
        const current = this._latest.get(update.extensionId);
        let terminal = current;
        if (current?.requestId !== update.requestId)
        {
            throw new Error("The settings operation was superseded before its result could be read.");
        }
        if (current.result.status === "running")
        {
            if (this._pending.has(update.extensionId))
            {
                throw new Error("A settings operation is already being observed.");
            }
            terminal = await new Promise<SettingsApplicationUpdate>(resolve =>
            {
                this._pending.set(update.extensionId, { requestId: update.requestId, resolve });
            });
        }
        if (terminal?.result.status === "completed")
        {
            return terminal.result;
        }
        throw new Error(terminal?.result.status === "failed" ? terminal.result.error : "Missing settings result.");
    }
}
