import { SvelteMap, SvelteSet } from "svelte/reactivity";

import type { SettingsEditResult } from "./SettingsEditResult";
import type { SettingsWriteResult } from "./SettingsWriteResult";
import type { OperationProgress } from "../types/OperationProgress";

/**
 * Owns field edits and accepted operations across navigation and native hiding.
 * One accepted edit per field bounds the queue; the backend owns execution policy.
 */
export class SettingsState<T extends object>
{
    values: T = $state({} as T);
    saved: T = $state.raw({} as T);
    effective: T | null = $state.raw(null);
    readonly phase = new SvelteMap<keyof T, "queued" | "running">();
    readonly progress = new SvelteMap<keyof T, OperationProgress>();
    readonly startedAt = new SvelteMap<keyof T, number>();
    readonly errors = new SvelteMap<keyof T, string>();
    private readonly _write: (
        key: keyof T,
        value: T[keyof T]
    ) => Promise<SettingsWriteResult<T>>;

    private readonly _prepare: (key: keyof T, draft: T[keyof T]) => SettingsEditResult<T[keyof T]>;
    private readonly _notify: (error: string) => void;
    private _pending = Promise.resolve();
    private readonly _uncommitted = new SvelteSet<keyof T>();
    private readonly _drafts = new SvelteSet<keyof T>();
    private readonly _failed = new SvelteSet<keyof T>();
    private _confirmed: T;
    private _observation: (() => SettingsWriteResult<T>) | undefined;
    private _refreshing: Promise<void> | undefined;

    constructor(
        initial: SettingsWriteResult<T>,
        write: (key: keyof T, value: T[keyof T]) => Promise<SettingsWriteResult<T>>,
        prepare: (key: keyof T, draft: T[keyof T]) => SettingsEditResult<T[keyof T]> = (_key, value) => ({ value }),
        notify: (error: string) => void = () =>
        {}
    )
    {
        this.values = structuredClone(initial.values);
        this._confirmed = structuredClone(initial.values);
        if (initial.error)
        {
            for (const key of Object.keys(initial.values) as Array<keyof T>)
            {
                this._failed.add(key);
            }
        }
        this.saved = structuredClone(initial.saved);
        this.effective = structuredClone(initial.effective);
        this._write = write;
        this._prepare = prepare;
        this._notify = notify;
    }

    edit<K extends keyof T>(key: K, value: T[K]): void
    {
        if (this.phase.get(key))
        {
            return;
        }
        this.values[key] = value;
        this._uncommitted.add(key);
        this._drafts.add(key);
        this.errors.delete(key);
    }

    async change<K extends keyof T>(key: K, value: T[K]): Promise<void>
    {
        this.edit(key, value);
        await this.commit(key);
    }

    async commit(key: keyof T): Promise<void>
    {
        if (this.phase.has(key) || !this._uncommitted.delete(key))
        {
            return;
        }
        // Settings values are JSON data; snapshotting cannot change their schema type.
        const snapshot: unknown = $state.snapshot(this.values[key]);
        // Compare and submit the same validated value. Input text remains a draft
        // until this boundary, including incomplete numbers and nested edits.
        const prepared = this._prepare(key, snapshot as T[keyof T]);
        if ("error" in prepared)
        {
            this._failure(key, prepared.error);
            return;
        }
        const value = prepared.value;
        // Presentation values come from the backend's persistence policy. Saved values
        // alone cannot prove a no-op after application succeeded but persistence failed.
        if (
            JSON.stringify(value) === JSON.stringify(this.saved[key])
            && JSON.stringify(value) === JSON.stringify(this._confirmed[key])
            && !this._failed.has(key)
            && !this._refreshing
        )
        {
            this._drafts.delete(key);
            if (JSON.stringify(snapshot) !== JSON.stringify(value))
            {
                this.values[key] = value;
            }
            return;
        }
        this._drafts.delete(key);
        this.errors.delete(key);
        this.phase.set(key, "queued");
        this.startedAt.set(key, Date.now());
        this._pending = this._pending.then(async () => this._run(key, async () => this._write(key, value)));
        await this._pending;
    }

    /**
     * Reattaches a running operation from the backend snapshot before input is enabled.
     */
    resume(key: keyof T, completion: Promise<SettingsWriteResult<T>>): void
    {
        this.phase.set(key, "running");
        this._pending = this._run(key, async () => completion);
    }

    commitEdits(): void
    {
        for (const key of this._uncommitted)
        {
            void this.commit(key);
        }
    }

    /**
     * Serializes an external-state read with writes without locking untouched controls.
     * Repeated activation signals share the same read; queued edits keep their draft.
     */
    async refresh(read: () => Promise<SettingsWriteResult<T>>): Promise<void>
    {
        if (this._refreshing)
        {
            return this._refreshing;
        }
        const refresh = this._pending.then(async () =>
        {
            this._reconcile(await read());
        });
        this._pending = refresh.catch(() =>
        {});
        this._refreshing = refresh.finally(() =>
        {
            this._refreshing = undefined;
        });
        return this._refreshing;
    }

    /**
     * Applies the latest authoritative observation after accepted edits settle.
     * The reader is evaluated at delivery so coalesced lifecycle updates stay current.
     */
    observe(read: () => SettingsWriteResult<T>): void
    {
        const queued = this._observation !== undefined;
        this._observation = read;
        if (!queued)
        {
            this._pending = this._pending.then(() =>
            {
                const latest = this._observation;
                this._observation = undefined;
                if (latest)
                {
                    this._reconcile(latest());
                }
            });
        }
    }

    private async _run(key: keyof T, write: () => Promise<SettingsWriteResult<T>>): Promise<void>
    {
        this.phase.set(key, "running");
        this.startedAt.set(key, this.startedAt.get(key) ?? Date.now());
        try
        {
            const result = await write();
            this._reconcile(result, key);
            this._failed.delete(key);
            if (result.error)
            {
                this._failed.add(key);
                this._failure(key, result.error);
            }
        }
        catch (error)
        {
            // Transport failure provides no evidence of application or rollback.
            this.effective = null;
            this.values[key] = structuredClone(this.saved[key]);
            this._failed.add(key);
            this._failure(key, String(error));
        }
        finally
        {
            this.phase.delete(key);
            this.progress.delete(key);
            this.startedAt.delete(key);
        }
    }

    private _reconcile(result: SettingsWriteResult<T>, completed?: keyof T): void
    {
        this.saved = result.saved;
        this.effective = result.effective;
        this._confirmed = result.values;
        for (const field of Object.keys(result.values) as Array<keyof T>)
        {
            // Invalid drafts remain owned by the user after their commit attempt.
            if (
                (field === completed || (!this.phase.has(field) && !this._drafts.has(field)))
                && JSON.stringify(this.values[field]) !== JSON.stringify(result.values[field])
            )
            {
                this.values[field] = structuredClone(result.values[field]);
            }
        }
    }

    private _failure(key: keyof T, error: string): void
    {
        this.errors.set(key, error);
        console.error("Settings operation failed", error);
        this._notify(error);
    }
}
