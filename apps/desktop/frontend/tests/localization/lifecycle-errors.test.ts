import { expect, test } from "vitest";

import { hostResources } from "../../src/generated/host-resources";
import { MAX_LIFECYCLE_DIAGNOSTIC_BYTES } from "../../src/generated/localization-limits";
import { lifecycleErrorText, lifecycleFailure } from "../../src/localization/lifecycle-errors";
import { ResourceSet } from "../../src/localization/ResourceSet";
import { settingsErrorText, settingsFailure } from "../../src/settings/error-presentation";

import type { LifecycleError } from "../../src/generated/LifecycleError";
import type { LifecycleFailure } from "../../src/generated/LifecycleFailure";
import type { LocalizedText } from "../../src/generated/LocalizedText";

const resources = new ResourceSet(hostResources);
const failure: LifecycleFailure = {
    error: {
        kind: "extension",
        error: {
            error: { code: "initialize_failed", message: { message: "failed" }, detail: "EACCES: original" },
            messages: { en: "Could not initialize", "zh-Hans": "无法初始化" }
        }
    },
    diagnostic: "stderr: retained original output\n".repeat(1000)
};

test("every recovery phase retains both locale presentations and separate original diagnostics", () =>
{
    const original: LifecycleFailure = {
        error: { kind: "host", code: "extensionExited" },
        diagnostic: "first process"
    };
    const errors: LifecycleError[] = [
        { phase: "failed", failure },
        { phase: "restarting", failure },
        { phase: "stopping", failure },
        { phase: "restartFailed", original, failure },
        { phase: "restartLimitReached", original, failure },
        {
            phase: "recoveryStopped",
            original,
            failures: [failure, { error: { kind: "host", code: "catalogRetirementFailed" } }]
        }
    ];
    for (const error of errors)
    {
        for (const locale of ["en", "zh-Hans"] as const)
        {
            const result = lifecycleErrorText(error, locale, value => resources.format(locale, value, "macos", 2048));
            expect(result).toContain(locale === "en" ? "Could not initialize" : "无法初始化");
            expect(result).toContain("EACCES: original");
            expect(result).toContain(failure.diagnostic);
            if ("original" in error)
            {
                expect(result).toContain("first process");
            }
        }
    }
});

test("settings command failures keep their shape and bound supplemental output independently", () =>
{
    const text = (value: LocalizedText) => resources.format("zh-Hans", value, "macos", 2048);
    const retained = settingsFailure(failure);
    expect(retained).toEqual(failure);
    expect(retained).not.toBe(failure);
    expect(settingsErrorText(retained, "zh-Hans", text)).toContain("无法初始化\nEACCES: original");
    expect(settingsFailure(failure.error)).toEqual(failure.error);
    for (const diagnostic of ["界".repeat(MAX_LIFECYCLE_DIAGNOSTIC_BYTES), {}, null])
    {
        expect(lifecycleFailure({ ...failure, diagnostic }).error).toMatchObject({
            kind: "host",
            code: "extensionProtocol"
        });
    }
    expect(lifecycleErrorText({ phase: "recoveryStopped", original: failure, failures: [] }, "zh-Hans", text))
        .toContain("扩展返回了无效响应");
});
