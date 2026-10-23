import { MAX_EXPANDED_BYTES } from "../generated/localization-limits";

/**
 * Check the shared Unicode scalar, control-character, and output budget contract.
 */
export function validOutput(value: string, maximumChars: number, allowEmpty: boolean): boolean
{
    const byteLength = new TextEncoder().encode(value).byteLength;
    if (byteLength > MAX_EXPANDED_BYTES || (!allowEmpty && value.trim().length === 0))
    {
        return false;
    }
    // Protocol limits count Unicode scalars, not grapheme clusters or UTF-16 units.
    let count = 0;
    for (const character of value)
    {
        count += 1;
        const scalar = character.charCodeAt(0);
        if (
            count > maximumChars || ((scalar < 32 || (scalar >= 127 && scalar <= 159)) && !"\n\r\t".includes(character))
        )
        {
            return false;
        }
    }
    return true;
}
