import { mount } from "svelte";

import "./styles/global.css";

const target = document.getElementById("app");

if (!target)
{
    throw new Error("Nanika frontend mount point is missing");
}

// Both surfaces use the same build. Settings never mounts the launcher or opens
// its search session, and its component code is loaded only when requested.
if (new URLSearchParams(window.location.search).get("surface") === "settings")
{
    void import("./Settings.svelte").then(({ default: Settings }) => mount(Settings, { target }));
}
else
{
    void import("./App.svelte").then(({ default: App }) => mount(App, { target }));
}

if (import.meta.env.DEV && new URLSearchParams(window.location.search).get("surface") !== "settings")
{
    void import("./development").then(({ mountPerformanceMonitor }) =>
    {
        mountPerformanceMonitor();
    });
}
