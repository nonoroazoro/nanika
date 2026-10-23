import { TextPresentation } from "./TextPresentation";

import type { ResourceSet } from "./ResourceSet";
import type { OperationProgress } from "../generated/OperationProgress";
import type { SupportedLocale } from "../generated/SupportedLocale";
import type { TargetPlatform } from "../generated/TargetPlatform";

/**
 * Retains a task label independently of work-unit updates and language changes.
 */
export class ProgressPresentation
{
    readonly completed: number;
    readonly total: number | null;
    private readonly _value: OperationProgress["label"];
    private readonly _key: string;
    private readonly _text: TextPresentation;

    constructor(
        progress: OperationProgress,
        resources: ResourceSet | undefined,
        platform: TargetPlatform,
        previous?: ProgressPresentation
    )
    {
        if (
            !Number.isInteger(progress.completed) || progress.completed < 0 || progress.completed > 0xFFFFFFFF
            || (progress.total === null
                ? progress.completed !== 0
                : !Number.isInteger(progress.total)
                    || progress.total <= 0 || progress.total > 0xFFFFFFFF || progress.completed > progress.total)
        )
        {
            throw new Error("Invalid progress work units.");
        }
        this.completed = progress.completed;
        this.total = progress.total;
        this._value = progress.label;
        this._key = JSON.stringify([progress.label, resources?.identity, platform]);
        this._text = previous?._key === this._key
            ? previous._text
            : new TextPresentation(resources, platform, [{ value: progress.label, maximum: 128 }]);
        for (const locale of ["en", "zh-Hans"] as const)
        {
            if (new TextEncoder().encode(this._text.text(progress.label, locale)).length > 128)
            {
                throw new Error("Progress label exceeds 128 bytes.");
            }
        }
    }

    /**
     * Select prepared text without affecting the accepted operation.
     * @param locale Authoritative presentation language
     */
    label(locale: SupportedLocale): string
    {
        return this._text.text(this._value, locale);
    }
}
