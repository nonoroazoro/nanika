<script lang="ts">
const { src }: { src: string | null; } = $props();
let failed = $state<string | null>(null);
let loaded = $state<string | null>(null);
</script>

{#if !src || failed !== src}
    <img
        src={src ?? undefined}
        class:pending={!src || loaded !== src}
        alt=""
        aria-hidden="true"
        decoding="async"
        onload={event =>
        {
            const image = event.currentTarget;
            if (
                image instanceof HTMLImageElement && image.getAttribute("src") === src && image.complete
                && image.naturalWidth > 0
            )
            {
                loaded = src;
            }
        }}
        onerror={() =>
        {
            failed = src;
        }}
    />
{:else}
    <svg viewBox="0 0 32 32" fill="none" aria-hidden="true">
        <rect x="1" y="1" width="30" height="30" rx="7" fill="currentColor" opacity="0.12" />
        <path
            d="M13 8H8v6a3 3 0 1 0 0 6v4h6a3 3 0 1 1 6 0h4v-6a3 3 0 1 0 0-6V8h-5a3 3 0 1 0-6 0Z"
            stroke="currentColor"
            stroke-width="1.5"
        />
    </svg>
{/if}

<style>
img, svg { display: block; width: var(--icon-size); height: var(--icon-size); object-fit: contain; color: var(--text-secondary); }
.pending { visibility: hidden; }
</style>
