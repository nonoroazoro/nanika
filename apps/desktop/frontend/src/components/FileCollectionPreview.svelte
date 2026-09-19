<script lang="ts">
import { settledGroups } from "./settledPreviewGroups";
import type { DetailView } from "../types";
import fileIcon from "../assets/clipboard-file.png";
import CachedFileIcon from "./CachedFileIcon.svelte";

const { files, resourceOrigin, extensionId, groupKey }: {
    files: Extract<DetailView["content"], { kind: "files"; }>["files"];
    resourceOrigin: string;
    extensionId: string;
    groupKey: string;
} = $props();
let completion = $state<{ key: string; indices: number[]; } | null>(null);
const ready = $derived(
    (completion?.key === groupKey && completion.indices.length === files.length) || settledGroups.has(groupKey)
);
const handlers = $derived.by(() =>
{
    const key = groupKey;
    return files.map((_, index) => () => markReady(key, index));
});

function markReady(key: string, index: number): void
{
    if (key !== groupKey)
    {
        return;
    }
    const indices = completion?.key === key ? completion.indices : [];
    if (!indices.includes(index))
    {
        completion = { key, indices: [...indices, index] };
        if (completion.indices.length === files.length)
        {
            settledGroups.add(key);
        }
    }
}
</script>

<div class="collection-stack" class:ready aria-hidden="true">
    {#each files as file, index (index)}
        <span>
            <img class="placeholder" src={fileIcon} alt="" />
            <div class="artwork">
                <CachedFileIcon
                    reference={file.icon}
                    {resourceOrigin}
                    {extensionId}
                    collection
                    onReady={ready ? undefined : handlers[index]}
                />
            </div>
        </span>
    {/each}
</div>

<style>
.collection-stack { display: flex; isolation: isolate; width: min(100%, 15rem); height: 7rem; align-items: center; justify-content: center; }
.collection-stack > span { position: relative; display: grid; place-items: center; flex: 0 0 5.5rem; height: 5.5rem; min-width: 0; margin: 0 -0.5rem; }
.collection-stack > span:first-child { transform: translateY(0.25rem) rotate(-4deg); }
.collection-stack > span:last-child { transform: translateY(0.25rem) rotate(4deg); }
.collection-stack > span:nth-child(1) { z-index: 0; }
.collection-stack > span:nth-child(2) { z-index: 1; }
.collection-stack > span:nth-child(3) { z-index: 2; }
.placeholder { position: absolute; inset: 0; width: 100%; height: 100%; object-fit: contain; }
.artwork { width: 100%; height: 100%; visibility: hidden; }
.ready .artwork { visibility: visible; }
.ready .placeholder { visibility: hidden; }
</style>
