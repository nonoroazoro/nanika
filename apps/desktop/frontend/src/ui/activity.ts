import { readable } from "svelte/store";

import type { UiActivity } from "../types/UiActivity";

/**
 * Shares native-document activity observation across a WebView's controls.
 * This reports activity only; Tauri owns launcher hiding.
 */
export const uiActivity = readable<UiActivity>({ visible: false, focused: false }, set =>
{
    const update = (): void =>
    {
        set({ visible: document.visibilityState === "visible", focused: document.hasFocus() });
    };
    update();
    document.addEventListener("visibilitychange", update);
    window.addEventListener("focus", update);
    window.addEventListener("blur", update);
    return () =>
    {
        document.removeEventListener("visibilitychange", update);
        window.removeEventListener("focus", update);
        window.removeEventListener("blur", update);
    };
});
