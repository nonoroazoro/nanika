import type { ButtonProps } from "./ButtonProps";

export type DismissButtonProps = {
    label: string;
} & Omit<ButtonProps, "aria-label" | "children" | "variant">;
