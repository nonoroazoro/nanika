import type { ConfirmationContent } from "./ConfirmationContent";
import type { ResultIcon } from "../generated/ResultIcon";

export interface ConfirmationRequest
{
    content: () => ConfirmationContent;
    destructive?: boolean;
    icon?: ResultIcon | null;
}
