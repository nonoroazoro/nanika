import { ResourceSet } from "./ResourceSet";
import { MAX_CACHED_RESOURCE_SETS, MAX_RESOURCE_LOADS } from "../generated/localization-limits";

import type { LocalizationResourceRequest } from "../generated/LocalizationResourceRequest";
import type { PreparedResources } from "../generated/PreparedResources";

/**
 * Bounded reuse between surfaces. Each mounted surface retains the returned immutable handle.
 * Eviction cannot change an open view, and an authorization change requires a new host read.
 */
export class ResourceCache<Request extends LocalizationResourceRequest>
{
    private readonly _read: (request: Request) => Promise<PreparedResources>;
    private readonly _cached = new Map<string, ResourceSet>();
    private readonly _authorized = new Set<string>();
    private readonly _pending = new Map<string, Promise<ResourceSet>>();

    constructor(read: (request: Request) => Promise<PreparedResources>)
    {
        this._read = read;
    }

    /**
     * Prepare all bundles before the caller publishes a new presentation.
     *
     * @param request Host-supplied scope with the requesting session and instance
     */
    async load(request: Request): Promise<ResourceSet>
    {
        const resourceKey = JSON.stringify([request.scope.extensionId, request.scope.identity]);
        const key = JSON.stringify([
            request.surface,
            request.sessionId,
            request.surface === "launcher" ? request.instanceId : null,
            request.scope.extensionId,
            request.scope.identity
        ]);
        const cached = this._cached.get(resourceKey);
        if (cached && this._authorized.has(key))
        {
            this._remember(key, resourceKey, cached);
            return cached;
        }
        const pending = this._pending.get(key);
        if (pending)
        {
            return pending;
        }
        if (this._pending.size >= MAX_RESOURCE_LOADS)
        {
            throw Object.assign(
                new Error("Too many localization resource loads are pending. Try again after they finish."),
                {
                    kind: "host",
                    code: "localizationBusy"
                }
            );
        }
        const owned = { ...request, scope: { ...request.scope } };
        // Start after registration so synchronous reader failures also release the pending slot.
        const loading = Promise.resolve().then(async () =>
        {
            const prepared = await this._read(owned);
            if (prepared.identity !== owned.scope.identity)
            {
                throw Object.assign(new Error("The localization response belongs to a different package."), {
                    kind: "host",
                    code: "localizationPackageChanged"
                });
            }
            const resources = this._cached.get(resourceKey) ?? new ResourceSet(prepared);
            this._remember(key, resourceKey, resources);
            return resources;
        }).finally(() =>
        {
            this._pending.delete(key);
        });
        this._pending.set(key, loading);
        return loading;
    }

    private _remember(key: string, resourceKey: string, resources: ResourceSet): void
    {
        this._cached.delete(resourceKey);
        this._cached.set(resourceKey, resources);
        this._authorized.delete(key);
        this._authorized.add(key);
        // Authority changes require a host read, but identical package content needs only one parse.
        for (const entries of [this._cached, this._authorized])
        {
            if (entries.size > MAX_CACHED_RESOURCE_SETS)
            {
                const oldest = entries.keys().next();
                if (!oldest.done)
                {
                    entries.delete(oldest.value);
                }
            }
        }
    }
}
