import { mount, unmount } from "svelte";

import PerformanceMonitor from "./PerformanceMonitor.svelte";

/**
 * Mount the development monitor independently of the application's view state.
 */
export function mountPerformanceMonitor(): void
{
    const target = document.createElement("div");
    target.id = "nanika-development-monitor";
    document.body.append(target);
    const component = mount(PerformanceMonitor, { target });
    import.meta.hot?.dispose(() =>
    {
        void unmount(component).then(() =>
        {
            target.remove();
        });
    });
}
