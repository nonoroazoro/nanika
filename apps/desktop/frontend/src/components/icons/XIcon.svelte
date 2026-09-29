<script lang="ts">
import Icon from "./Icon.svelte";
import type { IconProps } from "../../types/IconProps";

const props: IconProps = $props();
</script>

<Icon {...props}>
    <path d="M18 6 6 18" />
    <path d="m6 6 12 12" />
</Icon>
