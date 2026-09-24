<script lang="ts">
import type { TextareaProps } from "../types/TextareaProps";

let { ref = $bindable(), value = $bindable(""), variant = "field", class: className, ...attributes }: TextareaProps =
    $props();
</script>

<textarea
    {...attributes}
    bind:this={ref}
    bind:value
    class={["ui-textarea", variant === "field" && "control-field", className]}
    data-variant={variant}
></textarea>

<style>
.ui-textarea { display: block; width: 100%; color: var(--text-primary); font: inherit; line-height: 1.45; }
.ui-textarea[data-variant="plain"] { margin: 0; resize: none; appearance: none; padding: 0; border: 0; border-radius: 0; background: transparent; font-size: var(--font-row); }
</style>
