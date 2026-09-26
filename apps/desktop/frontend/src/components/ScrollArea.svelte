<script lang="ts">
import { ScrollArea } from "bits-ui";
import { onDestroy } from "svelte";
import type { ComponentProps, Snippet } from "svelte";

import { uiActivity } from "../ui/activity";

interface Props
{
    children: Snippet;
    class?: string;
    style?: string;
    onscroll?: (event: UIEvent & { currentTarget: EventTarget & HTMLDivElement; }) => void;
    viewport?: HTMLDivElement | null;
    viewportClass?: string;
    viewportProps?: Omit<ComponentProps<typeof ScrollArea.Viewport>, "children" | "ref">;
}

let {
    viewport = $bindable(null),
    viewportClass = "",
    viewportProps = {},
    children,
    class: className,
    onscroll,
    ...attributes
}: Props = $props();
let hovered = $state(false);
let scrolling = $state(false);
let dragging = $state(false);
let hideTimer: ReturnType<typeof setTimeout> | undefined;
const revealed = $derived($uiActivity.visible && (hovered || scrolling || dragging));

// Hidden documents end visual activity immediately, including an interrupted fade.
$effect(() =>
{
    if (!$uiActivity.visible)
    {
        hovered = false;
        dragging = false;
        _stopReveal();
    }
});
onDestroy(_stopReveal);

function _stopReveal(): void
{
    clearTimeout(hideTimer);
    hideTimer = undefined;
    scrolling = false;
}

function _scroll(event: UIEvent & { currentTarget: EventTarget & HTMLDivElement; }): void
{
    viewportProps.onscroll?.(event);
    onscroll?.(event);
    if (!$uiActivity.visible)
    {
        return;
    }
    scrolling = true;
    clearTimeout(hideTimer);
    hideTimer = setTimeout(_stopReveal, 500);
}

function _leave(): void
{
    hovered = false;
    _stopReveal();
}

function _release(): void
{
    dragging = false;
    _stopReveal();
}
</script>

<ScrollArea.Root
    {...attributes}
    type="always"
    class={["scroll-area", className]}
    data-active={$uiActivity.visible ? "" : undefined}
    data-revealed={revealed ? "" : undefined}
    data-dragging={dragging ? "" : undefined}
    onpointerenter={() =>
    {
        hovered = true;
    }}
    onpointerleave={_leave}
>
    <!-- Composed primitives share this element, including their ref attachments. -->
    <ScrollArea.Viewport
        {...viewportProps}
        bind:ref={viewport}
        class={["scroll-viewport", viewportClass, viewportProps.class]}
        onscroll={_scroll}
    >
        {@render children()}
    </ScrollArea.Viewport>
    <ScrollArea.Scrollbar
        orientation="vertical"
        class="scroll-track"
        onmousedown={event => event.preventDefault()}
        onpointerdown={event =>
        {
            if (event.button === 0)
            {
                dragging = true;
            }
        }}
        onpointerup={_release}
        onlostpointercapture={_release}
        onpointercancel={_release}
    >
        <ScrollArea.Thumb class="scroll-thumb" />
    </ScrollArea.Scrollbar>
</ScrollArea.Root>

<style>
:global(.scroll-area) { display: flex; flex: 1; flex-direction: column; min-width: 0; min-height: 0; overflow: hidden; }
:global(.scroll-viewport) { width: 100%; min-height: 0; max-height: inherit; flex: 1; }
:global(.scroll-track) {
  position: absolute;
  width: var(--scrollbar-size);
  padding: 0 1px;
  opacity: 0;
  pointer-events: none;
  touch-action: none;
  user-select: none;
  transition: opacity var(--motion-scrollbar-hide) linear;
}
:global(.scroll-area[data-revealed] > .scroll-track) {
  opacity: 1;
  pointer-events: auto;
  transition-duration: var(--motion-scrollbar-show);
}
:global(.scroll-thumb) { width: 100%; border-radius: 4px; background: var(--scrollbar-thumb); }
:global(.scroll-thumb:hover) { background: var(--scrollbar-thumb-hover); }
:global(.scroll-area[data-dragging] > .scroll-track > .scroll-thumb) { background: var(--scrollbar-thumb-active); }
:global(.scroll-area:not([data-active]) > .scroll-track) { transition: none; }
</style>
