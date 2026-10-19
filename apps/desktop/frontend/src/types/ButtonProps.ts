import type { Snippet } from "svelte";
import type { HTMLButtonAttributes } from "svelte/elements";

export type ButtonProps = {
    children?: Snippet;
    feedback?: boolean;
    focusRing?: boolean;
    pressFeedback?: boolean;
    ref?: HTMLButtonElement;
    size?: "action" | "content";
    variant?: "danger" | "ghost" | "outline" | "primary";
} & Omit<HTMLButtonAttributes, "children">;
