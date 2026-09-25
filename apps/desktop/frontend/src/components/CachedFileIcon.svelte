<script lang="ts">
import type { IconReference } from "../types";
import fileIcon from "../assets/clipboard-file.png";

const { reference, resourceOrigin, extensionId, preview = false, collection = false, onReady }: {
    reference: IconReference | null;
    resourceOrigin: string;
    extensionId: string;
    preview?: boolean;
    collection?: boolean;
    onReady?: () => void;
} = $props();
const source = $derived(
    reference
        ? `${resourceOrigin}/${extensionId}/${reference.key}/${preview || collection ? 512 : 128}.png`
        : fileIcon
);
let failedSource = $state<string | null>(null);
let element = $state<HTMLImageElement>();

$effect(() =>
{
    const notify = onReady;
    const image = element;
    if (!notify || !image)
    {
        return;
    }
    const requestedSource = failedSource === source ? fileIcon : source;
    let active = true;
    // Reused sources may emit no load event; decode explicitly and cancel on selection changes.
    void image.decode().then(() =>
    {
        if (active && image.getAttribute("src") === requestedSource)
        {
            notify();
        }
    }, () =>
    {
        if (active && image.getAttribute("src") === requestedSource)
        {
            handleFailure();
        }
    });
    return () =>
    {
        active = false;
    };
});

function handleFailure(): void
{
    if (!reference || failedSource === source)
    {
        onReady?.();
        return;
    }
    failedSource = source;
}
</script>

<img
    bind:this={element}
    class:preview
    class:collection
    src={failedSource === source ? fileIcon : source}
    alt=""
    aria-hidden="true"
    onerror={handleFailure}
/>

<style>
img { display: block; width: var(--icon-size); height: var(--icon-size); object-fit: contain; }
img.preview { width: min(100%, 16rem); height: 12rem; margin: 0 auto; }
img.collection { width: 100%; height: 100%; min-width: 0; min-height: 0; }
</style>
