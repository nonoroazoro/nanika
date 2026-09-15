import { Channel, invoke } from "@tauri-apps/api/core";

import type { NanikaBridge } from "./index";
import type { RootSearchSnapshot } from "../types";

export const tauriBridge: NanikaBridge = {
    openSession: async (listener, onError) =>
    {
        const updates = new Channel<RootSearchSnapshot>(snapshot =>
        {
            try
            {
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
    dismissLauncher: async () => invoke("dismiss_launcher")
};
