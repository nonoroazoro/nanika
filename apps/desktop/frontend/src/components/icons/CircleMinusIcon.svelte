<script lang="ts">
import Icon from "./Icon.svelte";
import type { IconProps } from "../../types/IconProps";

const props: IconProps = $props();
</script>

<Icon {...props}>
    <circle cx="12" cy="12" r="10" />
    <path d="M8 12h8" />
</Icon>
