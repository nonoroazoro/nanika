<script lang="ts">
import type { IconReference } from "../types";
import fileIcon from "../assets/clipboard-file.png";

const { reference, resourceOrigin, extensionId, preview = false, collection = false }: {
    reference: IconReference | null;
    resourceOrigin: string;
    extensionId: string;
    preview?: boolean;
    collection?: boolean;
} = $props();
const source = $derived(
    reference
        ? `${resourceOrigin}/${extensionId}/${reference.key}/${preview || collection ? 512 : 128}.png`
        : fileIcon
);
let failedSource = $state<string | null>(null);
</script>

<img
    class:preview
    class:collection
    src={failedSource === source ? fileIcon : source}
    alt=""
    aria-hidden="true"
    onerror={() =>
    {
        failedSource = source;
    }}
/>

<style>
img { display: block; width: var(--icon-size); height: var(--icon-size); object-fit: contain; }
img.preview { width: min(100%, 16rem); height: 12rem; margin: 0 auto; }
img.collection { width: 100%; height: 100%; min-width: 0; min-height: 0; }
</style>
