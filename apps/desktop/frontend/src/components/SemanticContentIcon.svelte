<script lang="ts">
import type { ViewItemIcon } from "../types";
import fileIcon from "../assets/clipboard-file.png";
import imageIcon from "../assets/clipboard-image.png";
import textIcon from "../assets/clipboard-text.png";

const { kind }: { kind: Extract<ViewItemIcon, string>; } = $props();
const icons: Record<Extract<ViewItemIcon, string>, string> = { files: fileIcon, image: imageIcon, text: textIcon };
</script>

<img src={icons[kind]} alt="" aria-hidden="true" />

<style>
img { display: block; width: var(--icon-size); height: var(--icon-size); object-fit: contain; }
</style>
