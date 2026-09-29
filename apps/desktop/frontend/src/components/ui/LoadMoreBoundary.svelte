<script lang="ts">
import { listPrefetchDistance } from "../../ui/list-loading";
import { uiActivity } from "../../ui/activity";
import Button from "./Button.svelte";

const { viewport, cursor, scope, busy, revision, onLoad }: {
    viewport: HTMLElement | null;
    cursor: string | null;
    scope: string;
    busy: boolean;
    revision: number;
    onLoad: (cursor: string) => Promise<number | null>;
} = $props();
let boundary = $state<HTMLDivElement>();
let requested = $state.raw<
    {
        scope: string;
        cursor: string;
        revision: number | null | undefined;
        position: number;
        measured: boolean;
    } | null
>(null);
let paused = $state(false);
const failed = $derived(requested?.scope === scope && requested.revision === null);

$effect(() =>
{
    const root = viewport;
    const target = boundary;
    const next = cursor;
    const identity = scope;
    const pending = requested;
    if (pending && pending.scope !== identity)
    {
        requested = null;
        paused = false;
        return;
    }
    if (
        !root || !target || !next || busy || !$uiActivity.visible
        || (pending?.scope === identity && (pending.revision === undefined
            || (pending.revision !== null && (pending.cursor === next || revision < pending.revision))))
    )
    {
        return;
    }
    if (pending && pending.revision !== null && !pending.measured)
    {
        // Invisible/control-only chunks can leave the boundary inside the prefetch
        // region forever. Require user intent when a completed read makes no progress.
        paused = _position(root, target) <= pending.position + 1;
        requested = { ...pending, measured: true };
        return;
    }
    if (paused || failed)
    {
        return;
    }
    // Fetch text ahead of the visible edge without resetting the native document.
    const observer = new IntersectionObserver(entries =>
    {
        if (!entries.some(entry => entry.isIntersecting))
        {
            return;
        }
        observer.disconnect();
        void _load(root, target, identity, next);
    }, { root, rootMargin: `0px 0px ${listPrefetchDistance(root.clientHeight)}px 0px` });
    observer.observe(target);
    return () => observer.disconnect();
});

async function _load(root: HTMLElement, target: HTMLElement, identity: string, next: string): Promise<void>
{
    const request = {
        scope: identity,
        cursor: next,
        revision: undefined as number | null | undefined,
        position: _position(root, target),
        measured: false
    };
    requested = request;
    paused = false;
    let receipt: number | null = null;
    try
    {
        receipt = await onLoad(next);
    }
    catch
    {
        // Failure is recoverable through the explicit retry control, never an auto loop.
    }
    if (requested === request)
    {
        requested = { ...request, revision: receipt };
    }
}

function _position(root: HTMLElement, target: HTMLElement): number
{
    return target.getBoundingClientRect().top - root.getBoundingClientRect().top + root.scrollTop;
}
</script>

{#if cursor}<div class="load-more-boundary" bind:this={boundary} aria-hidden="true"></div>{/if}
{#if cursor && (paused || failed)}
    <Button
        disabled={busy}
        onclick={() =>
        {
            if (viewport && boundary && cursor)
            {
                void _load(viewport, boundary, scope, cursor);
            }
        }}
    >{failed ? "Try again" : "Load more text"}</Button>
{/if}

<style>
.load-more-boundary { height: 1px; pointer-events: none; }
</style>
