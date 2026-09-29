<script lang="ts">
import { Motion } from "./Motion";
import { uiActivity } from "../../ui/activity";

const { container, target }: {
    container: HTMLElement | null;
    target: HTMLElement | null;
} = $props();
let surface = $state<HTMLDivElement>();
let _previousTarget: HTMLElement | null = null;

// Only DOM geometry is synchronized here. Selection and scrolling belong to the caller.
$effect(() =>
{
    const element = surface;
    if (!element)
    {
        return;
    }
    const geometry = new Motion(element);
    const visibility = new Motion(element);
    $effect(() =>
    {
        const owner = container;
        const destination = target;
        if (!$uiActivity.visible || !owner || !destination || !owner.contains(destination))
        {
            visibility.to({ opacity: "0" }, "--motion-enter");
            return;
        }
        const continuing = _previousTarget?.isConnected && getComputedStyle(element).opacity !== "0";
        _place(element, owner, destination, geometry, !continuing);
        visibility.to({ opacity: "1" }, "--motion-enter");
        _previousTarget = destination;
        const observer = new ResizeObserver(() => _place(element, owner, destination, geometry, true));
        observer.observe(owner);
        observer.observe(destination);
        return () => observer.disconnect();
    });
    return () =>
    {
        geometry.destroy();
        visibility.destroy();
    };
});

function _place(element: HTMLElement, owner: HTMLElement, destination: HTMLElement, motion: Motion, snap: boolean): void
{
    const bounds = owner.getBoundingClientRect();
    const rect = destination.getBoundingClientRect();
    const style = getComputedStyle(destination);
    const next = {
        transform: `translate(${rect.left - bounds.left - owner.clientLeft + destination.clientLeft}px, ${
            rect.top - bounds.top - owner.clientTop + destination.clientTop
        }px)`,
        width: `${destination.clientWidth}px`,
        height: `${destination.clientHeight}px`,
        "border-top-left-radius": style.borderTopLeftRadius,
        "border-top-right-radius": style.borderTopRightRadius,
        "border-bottom-right-radius": style.borderBottomRightRadius,
        "border-bottom-left-radius": style.borderBottomLeftRadius
    };
    const keys = Object.keys(next) as (keyof typeof next)[];
    if (keys.every(key => element.style.getPropertyValue(key) === next[key]))
    {
        return;
    }
    motion.to(next, "--motion-selection", snap);
}
</script>

<div class="selection-highlight" bind:this={surface} aria-hidden="true"></div>

<style>
.selection-highlight { position: absolute; top: 0; left: 0; z-index: 1; pointer-events: none; opacity: 0; background: var(--surface-selected); }
</style>
