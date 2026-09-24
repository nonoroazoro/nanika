import type { HTMLInputAttributes } from "svelte/elements";

export type InputProps = {
    ref?: HTMLInputElement;
    type?: "email" | "password" | "search" | "tel" | "text" | "url";
    value?: string;
    variant?: "embedded" | "field" | "search";
} & Omit<HTMLInputAttributes, "children" | "type" | "value">;
