<script lang="ts">
import type { Snippet } from "svelte";
import type { IconProps } from "../../types/IconProps";

const { size = 24, strokeWidth = 2, class: className = "", children }: IconProps & { children: Snippet; } = $props();
</script>

<svg
    width={size}
    height={size}
    class={className}
    viewBox="0 0 24 24"
    fill="none"
    stroke="currentColor"
    stroke-width={strokeWidth}
    stroke-linecap="round"
    stroke-linejoin="round"
    aria-hidden="true"
    focusable="false"
>
    {@render children()}
</svg>

<style>
svg { display: block; }
</style>
