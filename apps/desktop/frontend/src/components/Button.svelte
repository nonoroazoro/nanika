<script lang="ts">
import { Button } from "bits-ui";
import type { ButtonProps } from "../types/ButtonProps";

let { ref = $bindable(), type = "button", variant = "ghost", class: className, children, ...attributes }: ButtonProps =
    $props();
</script>

<Button.Root
    {...attributes}
    {type}
    bind:ref={() => ref ?? null, element =>
    {
        ref = element instanceof HTMLButtonElement ? element : undefined;
    }}
    class={["ui-button", variant !== "ghost" && "control-button", variant === "primary" && "primary", className]}
    data-variant={variant}
>
    {@render children?.()}
</Button.Root>

<style>
:global(.ui-button[data-variant="danger"]) { color: var(--text-danger); border-color: var(--border-danger); }
:global(.ui-button[data-variant="danger"]:hover:not(:disabled)) { border-color: var(--border-danger-hover); background: var(--surface-danger-hover); }
</style>
