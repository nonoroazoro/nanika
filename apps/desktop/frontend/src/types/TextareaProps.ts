import type { HTMLTextareaAttributes } from "svelte/elements";

export type TextareaProps = {
    ref?: HTMLTextAreaElement;
    value?: string;
    variant?: "field" | "plain";
} & Omit<HTMLTextareaAttributes, "children" | "value">;
