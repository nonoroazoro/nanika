<script lang="ts">
import type { CommandIcon } from "../types";

const { kind }: { kind: CommandIcon; } = $props();
</script>

<span class={`tile ${kind}`} aria-hidden="true">
    <svg viewBox="0 0 24 24">
        <!-- Glyph artwork from Lucide Icons, Copyright (c) 2026 Lucide Icons and Contributors, ISC License. -->
        <rect width="8" height="4" x="8" y="2" rx="1" ry="1" />
        <path d="M16 4h2a2 2 0 0 1 2 2v14a2 2 0 0 1-2 2H6a2 2 0 0 1-2-2V6a2 2 0 0 1 2-2h2" />
    </svg>
</span>

<style>
.tile {
  display: grid;
  width: var(--command-icon-size);
  height: var(--command-icon-size);
  place-items: center;
  border: 1px solid light-dark(rgba(58, 83, 145, 0.18), rgba(153, 174, 225, 0.16));
  border-radius: var(--command-icon-radius);
  background: light-dark(#718dca, #6076ad);
  color: #fff;
}

svg {
  display: block;
  width: var(--command-icon-glyph-size);
  height: var(--command-icon-glyph-size);
  fill: none;
  stroke: currentColor;
  stroke-width: 2;
  stroke-linecap: round;
  stroke-linejoin: round;
}
</style>
