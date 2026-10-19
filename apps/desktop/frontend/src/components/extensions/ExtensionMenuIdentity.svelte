<script lang="ts">
import ExtensionIcon from "./ExtensionIcon.svelte";

const { name, iconUrl }: { name: string; iconUrl: string; } = $props();
</script>

<div class="extension-menu-identity" role="group" aria-label={name}>
    <span class="icon"><ExtensionIcon src={iconUrl} /></span>
    <span class="name">{name}</span>
</div>

<style>
.extension-menu-identity { display: flex; align-items: center; flex: none; height: var(--menu-footer-height); padding-inline-start: calc(var(--menu-anchor-inset) - 1px); padding-inline-end: var(--space-3); color: var(--text-secondary); cursor: default; user-select: none; }
.icon { --icon-size: 20px; display: flex; justify-content: center; width: var(--menu-anchor-width); flex: none; }
.name { min-width: 0; margin-inline-start: var(--space-2); overflow: hidden; text-overflow: ellipsis; white-space: nowrap; font-size: var(--font-meta); }
</style>
