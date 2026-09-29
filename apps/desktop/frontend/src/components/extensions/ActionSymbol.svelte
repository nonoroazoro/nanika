<script module lang="ts">
import type { Component } from "svelte";
import type { ActionIcon } from "../../types/ActionIcon";
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
</script>

<script lang="ts">
const { name }: { name: ActionIcon; } = $props();
const SymbolIcon = $derived(symbols[name]);
</script>

<SymbolIcon size="100%" />
