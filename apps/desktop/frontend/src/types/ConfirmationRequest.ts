import type { ResultIcon } from "../generated/ResultIcon";

export interface ConfirmationRequest
{
    title: string;
    label: string;
    message: string;
    destructive?: boolean;
    icon?: ResultIcon | null;
}
