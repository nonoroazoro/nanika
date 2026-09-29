<script lang="ts">
import AppMark from "../icons/AppMark.svelte";
import { styleMotion } from "../motion/style";

const { version }: { version: string; } = $props();
const expansion = styleMotion({
    properties: ["transform", "opacity", "clip-path"],
    scope: ".context-menu",
    enterFrom: { transform: "translateX(-8px)", opacity: "0", "clip-path": "inset(0 100% 0 0)" },
    duration: element =>
        element.closest(".context-menu")?.getAttribute("data-state") === "open"
            ? "--motion-menu-enter"
            : "--motion-menu-exit"
});
</script>

<div class="app-menu-brand" role="group" aria-label={`Nanika v${version}`}>
    <div class="brand-line">
        <span class="brand-mark" aria-hidden="true"><AppMark /></span>
        <span class="brand-expansion" aria-hidden="true" {@attach expansion}>
            <span class="brand-name">anika</span>
            <span class="brand-version">v{version}</span>
        </span>
    </div>
</div>

<style>
/* The footer fills the trigger's inner height, excluding the two border pixels. */
.app-menu-brand { display: flex; align-items: center; flex: none; height: var(--menu-footer-height); color: var(--text-secondary); white-space: nowrap; cursor: default; -webkit-user-select: none; user-select: none; }
/* Equal side tracks center N in the trigger footprint; the suffix overflows to the right. */
.brand-line { display: grid; grid-template-columns: minmax(0, 1fr) max-content minmax(0, 1fr); align-items: baseline; width: var(--menu-anchor-width); height: 20px; margin-inline-start: calc(var(--menu-anchor-inset) - 1px); }
.brand-mark { grid-column: 2; display: inline-flex; }
.brand-expansion { grid-column: 3; justify-self: start; display: flex; align-items: baseline; gap: var(--space-2); height: 20px; transform: translateX(0); opacity: 1; clip-path: inset(0 0 0 0); }
.brand-name { font-family: system-ui, sans-serif; font-size: var(--font-row); font-weight: 600; line-height: 20px; }
.brand-version { color: var(--text-tertiary); font-size: var(--font-control); line-height: 20px; font-variant-numeric: tabular-nums; }
:global(.context-menu[data-state="closed"]) .brand-expansion { transform: translateX(-8px); opacity: 0; clip-path: inset(0 100% 0 0); }
@media (prefers-reduced-motion: reduce) {
  .brand-expansion, :global(.context-menu[data-state="closed"]) .brand-expansion { transform: none; }
}
</style>
