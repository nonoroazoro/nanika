<script lang="ts">
import type { ContributionIcon } from "../types";
import clipboardHistoryIcon from "../assets/clipboard-history.png";

const { kind }: { kind: ContributionIcon; } = $props();
const icons: Record<ContributionIcon, string> = { clipboard: clipboardHistoryIcon };
</script>

<img src={icons[kind]} alt="" aria-hidden="true" />

<style>
img {
  display: block;
  width: var(--icon-size);
  height: var(--icon-size);
  object-fit: contain;
}
</style>
