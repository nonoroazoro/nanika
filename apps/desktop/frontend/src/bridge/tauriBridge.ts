import { Channel, invoke } from "@tauri-apps/api/core";

import type { NanikaBridge } from "./index";
import type { NavigationSnapshot, RootSearchSnapshot } from "../types";

// Merge Channel deltas into coherent snapshots, preserving unchanged object identities.
type SearchUpdate = {
    navigation: { current?: NavigationSnapshot["current"]; } & Omit<NavigationSnapshot, "current">;
    results?: RootSearchSnapshot["results"];
} & Omit<RootSearchSnapshot, "navigation" | "results">;

export const tauriBridge: NanikaBridge = {
    readContextMenu: async request => invoke("read_context_menu", { request }),
    invokeContextMenu: async (request, actionId, confirmed) =>
        invoke("invoke_context_menu", { request, actionId, confirmed }),
    openSession: async (listener, onError) =>
    {
        let previous: RootSearchSnapshot | null = null;
        const updates = new Channel<SearchUpdate>(update =>
        {
            try
            {
                if (!previous && (update.results === undefined || update.navigation.current === undefined))
                {
                    throw new Error("The first session update must include results and navigation.");
                }
                const snapshot: RootSearchSnapshot = {
                    ...update,
                    results: update.results ?? previous?.results ?? [],
                    navigation: {
                        ...update.navigation,
                        current: update.navigation.current === undefined
                            ? previous?.navigation.current ?? null
                            : update.navigation.current
                    }
                };
                previous = snapshot;
                listener(snapshot);
                void invoke("acknowledge_search", {
                    sessionId: snapshot.sessionId,
                    revision: snapshot.revision
                }).catch(onError);
            }
            catch (error)
            {
                onError(error);
            }
        });
        return invoke("open_session", { updates });
    },
    closeSession: async sessionId => invoke("close_session", { sessionId }),
    publishQuery: async request => invoke("publish_query", { request }),
    refreshSearch: async sessionId => invoke("refresh_search", { sessionId }),
    invokeCandidate: async request => invoke("invoke_candidate", { request }),
    viewEvent: async request => invoke("view_event", { request }),
    dismissLauncher: async () => invoke("dismiss_launcher"),
    openSettings: async () => invoke("open_settings")
};
