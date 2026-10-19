import { mount } from "svelte";

import ConfirmationDialog from "../src/components/ui/ConfirmationDialog.svelte";
import { uiActivity } from "../src/ui/activity";

import "../src/styles/global.css";

const errors: string[] = [];
Object.assign(window, { verificationErrors: errors });
window.addEventListener("error", event =>
{
    errors.push(event.message);
});
window.addEventListener("unhandledrejection", event =>
{
    errors.push(String(event.reason));
});

const target = document.getElementById("app");
if (!target)
{
    throw new Error("Verification root is missing.");
}
const params = new URLSearchParams(location.search);
const surface = params.get("surface") ?? "syntax";
document.documentElement.dataset.surface = surface === "settings" ? "settings" : "launcher";
uiActivity.subscribe(activity =>
{
    document.documentElement.dataset.uiActive = String(activity.visible && activity.focused);
});
mount(ConfirmationDialog, { target });
if (surface === "syntax")
{
    const { default: SyntaxSurface } = await import("./syntax/SyntaxSurface.svelte");
    mount(SyntaxSurface, { target });
}
else if (surface === "settings")
{
    const { default: Settings } = await import("../src/Settings.svelte");
    mount(Settings, { target });
}
else if (surface === "launcher")
{
    const { default: App } = await import("../src/App.svelte");
    mount(App, { target });
}
else
{
    throw new Error(`Unknown verification surface: ${surface}`);
}

if (new URLSearchParams(location.search).has("native"))
{
    void import("./native").then(async ({ runNative }) => runNative());
}
