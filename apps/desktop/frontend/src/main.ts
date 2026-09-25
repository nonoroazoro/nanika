import { mount } from "svelte";

import { uiActivity } from "./ui/activity";

import "./styles/global.css";

const stopActivity = uiActivity.subscribe(activity =>
{
    document.documentElement.dataset.uiActive = String(activity.visible && activity.focused);
});
import.meta.hot?.dispose(stopActivity);

const isSettings = new URLSearchParams(window.location.search).get("surface") === "settings";
document.documentElement.dataset.surface = isSettings ? "settings" : "launcher";

const target = document.getElementById("app");

if (!target)
{
    throw new Error("Nanika frontend mount point is missing");
}

// Load only the requested surface to keep Settings out of the launcher search session.
if (isSettings)
{
    void import("./Settings.svelte").then(({ default: Settings }) => mount(Settings, { target }));
}
else
{
    void import("./App.svelte").then(({ default: App }) => mount(App, { target }));
}

if (import.meta.env.DEV && !isSettings)
{
    void import("./development").then(({ mountPerformanceMonitor }) =>
    {
        mountPerformanceMonitor();
    });
}
