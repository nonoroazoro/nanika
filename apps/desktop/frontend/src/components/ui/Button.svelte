<script lang="ts">
import { createAttachmentKey } from "svelte/attachments";
import { buttonMotion } from "../motion/style";
import { Button } from "bits-ui";

import type { ButtonProps } from "../../types/ButtonProps";

const motionKey = createAttachmentKey();

let {
    feedback = true,
    ref = $bindable(),
    type = "button",
    variant = "ghost",
    class: className,
    children,
    ...attributes
}: ButtonProps = $props();
</script>

<Button.Root
    {...{ [motionKey]: feedback ? buttonMotion : undefined }}
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
:global(.ui-button[data-variant="danger"]:hover:not(:disabled, [aria-disabled="true"])) { border-color: var(--border-danger-hover); background: var(--surface-danger-hover); }
</style>
