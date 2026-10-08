<script module lang="ts">
import type { Component } from "svelte";
import type { ActionIcon } from "../../generated/ActionIcon";
import type { TargetPlatform } from "../../generated/TargetPlatform";
import type { IconProps } from "../../types/IconProps";
import FolderOpenIcon from "../icons/FolderOpenIcon.svelte";
import LockKeyholeIcon from "../icons/LockKeyholeIcon.svelte";
import LogOutIcon from "../icons/LogOutIcon.svelte";
import MonitorOffIcon from "../icons/MonitorOffIcon.svelte";
import MoonIcon from "../icons/MoonIcon.svelte";
import PowerIcon from "../icons/PowerIcon.svelte";
import RotateCwIcon from "../icons/RotateCwIcon.svelte";
import TrashIcon from "../icons/TrashIcon.svelte";

// Protocol values select compiled host components, never extension-supplied SVG.
const symbols: Record<ActionIcon, Component<IconProps>> = {
    "folder-open": FolderOpenIcon,
    "lock-keyhole": LockKeyholeIcon,
    "log-out": LogOutIcon,
    "monitor-off": MonitorOffIcon,
    "moon": MoonIcon,
    "power": PowerIcon,
    "rotate-cw": RotateCwIcon,
    "trash": TrashIcon
};

// Shared code points in Segoe Fluent Icons (Windows 11) and Segoe MDL2 Assets (Windows 10).
// https://learn.microsoft.com/windows/apps/design/iconography/segoe-fluent-icons-font
const windowsSymbols: Record<ActionIcon, string> = {
    "folder-open": "\uED25",
    "lock-keyhole": "\uE72E",
    "log-out": "\uF3B1",
    "monitor-off": "\uE7F4",
    "moon": "\uE708",
    "power": "\uE7E8",
    "rotate-cw": "\uE777",
    "trash": "\uE74D"
};
</script>

<script lang="ts">
const { name, platform }: { name: ActionIcon; platform: TargetPlatform; } = $props();
const SymbolIcon = $derived(symbols[name]);
</script>

{#if platform === "windows"}
    <span class="windows-icon" aria-hidden="true">{windowsSymbols[name]}</span>
{:else}
    <span class="action-icon" aria-hidden="true">
        <SymbolIcon size="1.125rem" strokeWidth={1.8} />
    </span>
{/if}

<style>
.windows-icon {
  display: grid;
  width: var(--icon-size);
  height: var(--icon-size);
  place-items: center;
  color: var(--text-primary);
  font-family: "Segoe Fluent Icons", "Segoe MDL2 Assets";
  font-size: 20px;
  font-weight: 400;
  line-height: 1;
}

.action-icon {
  display: grid;
  /* Native app assets include transparent margins inside the shared icon column. */
  width: calc(var(--icon-size) - 2px);
  height: calc(var(--icon-size) - 2px);
  place-items: center;
  border-radius: 25%;
  background: linear-gradient(to bottom, #a8a8ae, #828287 50%, #5b5b60);
  color: #fff;
  box-shadow: inset 0 0.5px 0 rgb(255 255 255 / 12%), 0 0.5px 1px rgb(0 0 0 / 12%);
}
</style>
