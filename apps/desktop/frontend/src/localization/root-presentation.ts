import { actionFields } from "./action-fields";
import { TextPresentation } from "./TextPresentation";
import { MAX_EXPANDED_BYTES } from "../generated/localization-limits";

import type { PresentationField } from "./PresentationField";
import type { ResourceCache } from "./ResourceCache";
import type { ApplicationSnapshot } from "../generated/ApplicationSnapshot";
import type { LocalizationResourceRequest } from "../generated/LocalizationResourceRequest";
import type { SearchResult } from "../generated/SearchResult";
import type { RootPresentation } from "../types/RootPresentation";

/**
 * Prepare the mounted root window while retaining resources beyond reusable-cache eviction.
 * Authority changes require a new host read; query updates keep existing immutable handles.
 */
export async function prepareRootPresentation(
    results: readonly SearchResult[],
    session: Pick<ApplicationSnapshot, "platform" | "sessionId">,
    previous: ReadonlyMap<string, RootPresentation>,
    resources: ResourceCache<Extract<LocalizationResourceRequest, { surface: "launcher"; }>>
): Promise<Map<string, RootPresentation>>
{
    const owners = new Map<string, { fields: PresentationField[]; result: SearchResult; }>();
    for (const result of results)
    {
        const owner = owners.get(result.extensionId) ?? { result, fields: [] };
        owner.fields.push({
            value: result.title,
            maximum: MAX_EXPANDED_BYTES,
            allowEmpty: typeof result.title === "string"
        });
        if (result.subtitle)
        {
            owner.fields.push({
                value: result.subtitle.text,
                maximum: MAX_EXPANDED_BYTES,
                allowEmpty: typeof result.subtitle.text === "string"
            });
        }
        owner.fields.push(...actionFields(result.actions));
        owners.set(result.extensionId, owner);
    }
    const prepared = new Map<string, RootPresentation>();
    // The mounted result window bounds retained owners. Sequential reads respect the shared pending-load limit.
    for (const [id, { result, fields }] of owners)
    {
        if (result.resourceScope && result.resourceScope.extensionId !== id)
        {
            throw new Error("Search resources do not match their owner.");
        }
        const retained = previous.get(id);
        let bundle = retained?.presentation.resources;
        if (
            retained?.sessionId !== session.sessionId || retained.instanceId !== result.instanceId
            || bundle?.identity !== result.resourceScope?.identity
        )
        {
            bundle = result.resourceScope
                ? await resources.load({
                    surface: "launcher",
                    sessionId: session.sessionId,
                    instanceId: result.instanceId,
                    scope: result.resourceScope
                })
                : undefined;
        }
        prepared.set(id, {
            sessionId: session.sessionId,
            instanceId: result.instanceId,
            presentation: new TextPresentation(bundle, session.platform, fields)
        });
    }
    return prepared;
}
