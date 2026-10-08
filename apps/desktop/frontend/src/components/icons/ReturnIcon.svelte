<script lang="ts">
import Icon from "./Icon.svelte";
import type { IconProps } from "../../types/IconProps";

const props: IconProps = $props();
</script>

<Icon {...props}>
    <path d="M19 5v9H5m5-5-5 5 5 5" />
</Icon>
