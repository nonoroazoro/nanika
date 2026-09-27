<script lang="ts">
import Icon from "./Icon.svelte";
import type { IconProps } from "../../types/IconProps";

const props: IconProps = $props();
</script>

<Icon {...props}>
    <path d="M12 2v10" />
    <path d="M18.4 6.6a9 9 0 1 1-12.77.04" />
</Icon>
