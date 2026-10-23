/**
 * A schema failure retains constraints and the exact property/index path, never draft content.
 */
export type SettingsValidationError =
    & (
        | {
            code: "arrayLength" | "integerMaximum" | "integerMinimum" | "integerMultiple" | "textBytes";
            constraint: number;
        }
        | {
            code:
                | "arrayRequired"
                | "booleanRequired"
                | "integerLimit"
                | "objectRequired"
                | "textRequired"
                | "unknownSetting"
                | "wholeNumber";
        }
    )
    & { path: Array<number | string>; };
