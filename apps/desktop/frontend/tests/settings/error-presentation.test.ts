import { expect, test } from "vitest";

import { hostResources } from "../../src/generated/host-resources";
import { ResourceSet } from "../../src/localization/ResourceSet";
import { settingsErrorText, settingsFailure } from "../../src/settings/error-presentation";

import type { ConfigurationSaveError } from "../../src/generated/ConfigurationSaveError";

test("configuration failure codes retain transaction meaning and original details in both languages", () =>
{
    const resources = new ResourceSet(hostResources);
    const failures: Array<[ConfigurationSaveError, string, string]> = [
        [
            { code: "rejected", cause: { kind: "host", code: "operationPending", detail: "busy" } },
            "not accepted",
            "未被接受"
        ],
        [
            { code: "unconfirmed", cause: { kind: "host", code: "configurationUnconfirmed", detail: "lost receipt" } },
            "unclear",
            "无法确认"
        ],
        [{ code: "saveFailed", detail: "disk full" }, "was not applied", "尚未应用"],
        [{ code: "appliedButNotSaved", detail: "disk full" }, "took effect", "已生效"],
        [{ code: "unavailable" }, "unavailable", "不可用"],
        [
            {
                code: "unconfirmedNotSaved",
                cause: { kind: "host", code: "configurationUnconfirmed", detail: "device disconnected" }
            },
            "not be confirmed",
            "无法确认"
        ],
        [
            { code: "savedButNotApplied", cause: { kind: "host", code: "extensionIo", detail: "permission denied" } },
            "was saved",
            "已保存"
        ]
    ];
    for (const [failure, english, chinese] of failures)
    {
        const original = structuredClone(failure);
        expect(settingsFailure(failure)).toEqual(failure);
        for (const [locale, expected] of [["en", english], ["zh-Hans", chinese]] as const)
        {
            for (const platform of ["macos", "windows"] as const)
            {
                const value = settingsErrorText(failure, locale, text =>
                    resources.format(locale, text, platform, 2048, false));
                expect(value).toContain(expected);
                if ("detail" in failure)
                {
                    expect(value.endsWith(`\n${failure.detail}`)).toBe(true);
                }
                if ("cause" in failure && failure.cause.kind === "host")
                {
                    expect(value.endsWith(`\n${failure.cause.detail}`)).toBe(true);
                }
                expect(failure).toEqual(original);
            }
        }
    }
});

test("command rejection decoding never infers a transaction result from prose or an invalid code", () =>
{
    const message = "The change was saved but could not be applied: OS error";
    expect(settingsFailure(message)).toEqual({ kind: "host", code: "invocationUnconfirmed", detail: message });
    expect(settingsFailure(new Error("disk full"))).toEqual({
        kind: "host",
        code: "invocationUnconfirmed",
        detail: "disk full"
    });
    for (
        const failure of [
            { code: "toString", detail: "x" },
            { code: "saveFailed", detail: 42 },
            { code: "saveFailed" },
            { code: "saveFailed", detail: "界".repeat(65536) },
            { code: "saveFailed", detail: "invalid\0diagnostic" }
        ]
    )
    {
        expect(settingsFailure(failure)).toMatchObject({ kind: "host", code: "extensionProtocol" });
    }
    const resources = new ResourceSet(hostResources);
    for (const locale of ["en", "zh-Hans"] as const)
    {
        const text = settingsErrorText(settingsFailure(message), locale, value =>
            resources.format(locale, value, "macos", 2048));
        expect(text).toContain(message);
        expect(text).toContain(locale === "en" ? "could not be confirmed" : "无法确认");
    }
});

test("settings actions retain admitted error presentation after their extension is gone", () =>
{
    const failure = settingsFailure({
        kind: "extension",
        error: {
            error: { code: "action_failed", message: { message: "action-failed" }, detail: "EACCES: original" },
            messages: { en: "Could not complete action", "zh-Hans": "无法完成操作" }
        }
    });
    for (const [locale, message] of [["en", "Could not complete action"], ["zh-Hans", "无法完成操作"]] as const)
    {
        expect(settingsErrorText(failure, locale, () =>
        {
            throw new Error("Retained action failures require no resource lookup");
        })).toBe(`${message}\nEACCES: original`);
    }
    expect(settingsFailure({ kind: "extension", error: { messages: {} } })).toEqual({
        kind: "host",
        code: "extensionProtocol",
        detail: "Invalid structured error presentation."
    });
});

test("configuration failures retain transaction facts and localized causes independently of package lifetime", () =>
{
    const source = {
        kind: "extension",
        error: {
            error: { code: "apply_failed", message: { message: "apply-failed" }, detail: "EACCES: original" },
            messages: { en: "Could not apply settings", "zh-Hans": "无法应用设置" }
        }
    };
    const resources = new ResourceSet(hostResources);
    for (const code of ["savedButNotApplied", "unconfirmedNotSaved"] as const)
    {
        const cause = structuredClone(source);
        const failure = settingsFailure({ code, cause });
        cause.error.messages.en = "Changed after delivery";
        for (const [locale, expected] of [["en", "Could not apply settings"], ["zh-Hans", "无法应用设置"]] as const)
        {
            const formatted = settingsErrorText(
                failure,
                locale,
                value => resources.format(locale, value, "macos", 2048)
            );
            expect(formatted).toContain(expected);
            expect(formatted).toContain("EACCES: original");
            expect(formatted).not.toContain("Changed after delivery");
            const transaction = code === "savedButNotApplied"
                ? { en: "was saved", "zh-Hans": "已保存" }
                : { en: "was not saved", "zh-Hans": "未保存" };
            expect(formatted).toContain(transaction[locale]);
        }
        for (const invalid of [undefined, null, "raw failure", { kind: "extension", error: { messages: {} } }])
        {
            expect(settingsFailure({ code, cause: invalid })).toEqual({
                code,
                cause: { kind: "host", code: "extensionProtocol", detail: "Invalid structured error presentation." }
            });
        }
    }
});
