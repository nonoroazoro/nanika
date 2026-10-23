import type { PresentationField } from "./PresentationField";
import type { Action } from "../generated/Action";

/**
 * Collect presentation only, preserving action identity and invocation policy.
 * @param actions Accepted actions from one package
 */
export function actionFields(actions: readonly Action[]): PresentationField[]
{
    return actions.flatMap(action =>
    {
        const fields: PresentationField[] = [{ value: action.title, maximum: 128 }];
        if (action.confirmation_title)
        {
            fields.push({ value: action.confirmation_title, maximum: 128 });
        }
        if (action.confirmation_description)
        {
            fields.push({ value: action.confirmation_description, maximum: 512 });
        }
        if (action.group)
        {
            fields.push({ value: action.group.title, maximum: 128 });
        }
        return fields;
    });
}
