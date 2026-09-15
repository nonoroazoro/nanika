<script lang="ts">
import type { ViewItemIcon } from "../types";

const { kind }: { kind: ViewItemIcon; } = $props();
</script>

<svg class={kind} viewBox="0 0 24 24" aria-hidden="true">
    <!-- Glyph artwork from Lucide Icons, Copyright (c) 2026 Lucide Icons and Contributors, ISC License. -->
    <g class="glyph">
        {#if kind === "image"}
            <rect width="18" height="18" x="3" y="3" rx="2" ry="2" />
            <circle cx="9" cy="9" r="2" />
            <path d="m21 15-3.086-3.086a2 2 0 0 0-2.828 0L6 21" />
        {:else}
            <path d="M6 22a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2h8a2.4 2.4 0 0 1 1.704.706l3.588 3.588A2.4 2.4 0 0 1 20 8v12a2 2 0 0 1-2 2z" />
            <path d="M14 2v5a1 1 0 0 0 1 1h5" />
            {#if kind === "text"}
                <path d="M10 9H8M16 13H8M16 17H8" />
            {/if}
        {/if}
    </g>
</svg>

<style>
svg { display: block; width: 1.25rem; height: 1.25rem; }
.glyph { fill: none; stroke: var(--icon-color); stroke-width: 2; stroke-linecap: round; stroke-linejoin: round; }
.text { --icon-color: light-dark(#6656cf, #a69cff); }
.files { --icon-color: light-dark(#2874ca, #80baff); }
.image { --icon-color: light-dark(#16866b, #62cbae); }
</style>
