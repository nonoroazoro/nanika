<script lang="ts">
import type { InputProps } from "../types/InputProps";

let { ref = $bindable(), value = $bindable(""), type = "text", variant = "field", class: className, ...attributes }:
    InputProps = $props();
</script>

<input
    {...attributes}
    {type}
    bind:this={ref}
    bind:value
    class={["ui-input", variant === "field" && "control-field", className]}
    data-variant={variant}
/>

<style>
.ui-input { min-width: 0; color: var(--text-primary); caret-color: var(--accent); }
.ui-input::placeholder { color: var(--text-tertiary); opacity: 1; }
.ui-input[data-variant="search"] { width: 100%; height: 100%; padding: 0; border: 0; background: transparent; font: inherit; font-size: var(--font-search); }
.ui-input[data-variant="embedded"] { width: 100%; padding: var(--control-padding); border: 0; background: transparent; font-size: var(--font-control); }
</style>
