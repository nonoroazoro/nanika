import type { ActionIcon } from "./ActionIcon";

export interface Action
{
    id: string;
    title: string;
    icon?: ActionIcon;
    enabled: boolean;
    allow_default_execution: boolean;
    group: string | null;
    confirmation_title?: string;
    style: "destructive" | "primary" | "secondary";
}
