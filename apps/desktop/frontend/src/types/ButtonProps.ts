import type { Snippet } from "svelte";
import type { HTMLButtonAttributes } from "svelte/elements";

export type ButtonProps = {
    children?: Snippet;
    feedback?: boolean;
    ref?: HTMLButtonElement;
    variant?: "danger" | "ghost" | "outline" | "primary";
} & Omit<HTMLButtonAttributes, "children">;
