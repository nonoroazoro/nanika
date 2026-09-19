<script lang="ts">
import type { ContributionIcon } from "../types";
import clipboardHistoryIcon from "../assets/clipboard-history.png";

const { kind }: { kind: ContributionIcon; } = $props();
</script>

{#if kind === "clipboard"}
    <img src={clipboardHistoryIcon} alt="" aria-hidden="true" />
{:else}
    <svg class={kind} viewBox="0 0 32 32" fill="none" aria-hidden="true">
        <rect class="tile" x="1" y="1" width="30" height="30" rx="7" />
        <g stroke="white" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round">
            {#if kind === "applications"}
                <rect x="8" y="8" width="6" height="6" rx="1.5" />
                <rect x="18" y="8" width="6" height="6" rx="1.5" />
                <rect x="8" y="18" width="6" height="6" rx="1.5" />
                <rect x="18" y="18" width="6" height="6" rx="1.5" />
            {:else if kind === "calculator"}
                <path d="M8 11h7m-3.5-3.5v7M19 11h5M9 20l5 5m0-5-5 5M19 20h5m-5 4h5" />
            {:else if kind === "command"}
                <path d="m8 10 6 6-6 6m10 0h6" />
            {:else if kind === "script"}
                <path d="m11 11-5 5 5 5m10-10 5 5-5 5M18 8l-4 16" />
            {:else if kind === "extension"}
                <path d="M13 8H8v6a3 3 0 1 0 0 6v4h6a3 3 0 1 1 6 0h4v-6a3 3 0 1 0 0-6V8h-5a3 3 0 1 0-6 0Z" />
            {/if}
        </g>
    </svg>
{/if}

<style>
img, svg {
  display: block;
  width: var(--icon-size);
  height: var(--icon-size);
  object-fit: contain;
}
.tile { fill: #687384; }
.applications .tile { fill: #3478d4; }
.calculator .tile { fill: #c57723; }
.command .tile { fill: #404b5e; }
.script .tile { fill: #7960bc; }
</style>
