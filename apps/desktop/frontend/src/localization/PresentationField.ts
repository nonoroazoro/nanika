import type { ViewMetadataValue } from "../generated/ViewMetadataValue";

export interface PresentationField
{
    value: ViewMetadataValue;
    maximum: number;
    allowEmpty?: boolean;
}
