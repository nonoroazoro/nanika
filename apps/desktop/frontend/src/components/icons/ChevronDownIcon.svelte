<script lang="ts">
import Icon from "./Icon.svelte";
import type { IconProps } from "../../types/IconProps";

const props: IconProps = $props();
</script>

<Icon {...props}>
    <path d="m6 9 6 6 6-6" />
</Icon>
