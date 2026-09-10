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
                if (import.meta.env.VITE_PERFORMANCE_OBSERVATION === "1")
                {
                    document.dispatchEvent(new CustomEvent("nanika:search-received", { detail: snapshot }));
                }
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
    invokeCandidate: async request => invoke("invoke_candidate", { request }),
    dismissLauncher: async () => invoke("dismiss_launcher")
};
