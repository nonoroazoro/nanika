import type { Snippet } from "svelte";

import type { ButtonProps } from "./ButtonProps";

export type TruncatedButtonProps = {
    icon?: Snippet;
    label: string;
} & Omit<ButtonProps, "children" | "title">;
