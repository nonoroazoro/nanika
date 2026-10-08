<script lang="ts">
import AppMark from "../icons/AppMark.svelte";
import { brandMotion } from "./brand-motion";

const { version }: { version: string; } = $props();
</script>

<div class="app-menu-brand" role="group" aria-label={`Nanika v${version}`}>
    <div class="brand-line">
        <span class="brand-mark" aria-hidden="true"><AppMark /></span>
        {#key version}
            <span class="brand-expansion" aria-hidden="true" {@attach brandMotion}>
                <span class="brand-name">{#each Array.from("anika") as letter, index (index)}<span class="brand-glyph">{
                            letter
                        }</span>{/each}</span>
                <span class="brand-version">{#each Array.from(`v${version}`) as letter, index (index)}<span
                            class="brand-glyph"
                        >{letter}</span>{/each}</span>
            </span>
        {/key}
    </div>
</div>

<style>
/* The footer fills the trigger's inner height, excluding the two border pixels. */
.app-menu-brand { display: flex; align-items: center; flex: none; height: var(--menu-footer-height); color: var(--text-secondary); white-space: nowrap; cursor: default; -webkit-user-select: none; user-select: none; }
/* Equal side tracks center N in the trigger footprint; the suffix overflows to the right. */
.brand-line { display: grid; grid-template-columns: minmax(0, 1fr) max-content minmax(0, 1fr); align-items: baseline; width: var(--menu-anchor-width); height: 20px; margin-inline-start: calc(var(--menu-anchor-inset) - 1px); }
.brand-mark { grid-column: 2; display: inline-flex; }
.brand-expansion { --motion-brand-enter: 220ms; grid-column: 3; justify-self: start; display: flex; align-items: baseline; gap: var(--space-2); height: 20px; }
.brand-glyph { display: inline-block; transform: translate(0, 0); opacity: 1; }
.brand-name { font-family: system-ui, sans-serif; font-size: var(--font-row); font-weight: 600; line-height: 20px; }
.brand-version { color: var(--text-tertiary); font-size: var(--font-control); line-height: 20px; font-variant-numeric: tabular-nums; }
:global(.context-menu[data-state="closed"]) .brand-glyph { transform: translate(-4px, 2px); opacity: 0; }
@media (prefers-reduced-motion: reduce) {
  .brand-glyph, :global(.context-menu[data-state="closed"]) .brand-glyph { transform: none; }
}
</style>
