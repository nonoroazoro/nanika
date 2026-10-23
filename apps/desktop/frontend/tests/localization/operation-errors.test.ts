import { expect, test } from "vitest";

import { hostResources } from "../../src/generated/host-resources";
import { MAX_ERROR_DETAIL_BYTES, MAX_ERROR_MESSAGE_CHARS } from "../../src/generated/localization-limits";
import { operationErrorText, operationFailure } from "../../src/localization/operation-errors";
import { ResourceSet } from "../../src/localization/ResourceSet";

import type { LocalizedText } from "../../src/generated/LocalizedText";

const resources = new ResourceSet(hostResources);
const failure = {
    kind: "extension",
    error: {
        error: {
            code: "read_failed",
            message: { message: "read-failed", args: { name: "private.txt" } },
            detail: "EACCES: /Original/私密.txt"
        },
        messages: { en: "Could not read private.txt", "zh-Hans": "无法读取 private.txt" }
    }
};

test("retained extension errors switch language without any package resource lookup", () =>
{
    const retained = operationFailure(failure, "textReadFailed");
    for (
        const [locale, expected] of [["en", "Could not read private.txt"], ["zh-Hans", "无法读取 private.txt"]] as const
    )
    {
        expect(operationErrorText(retained, locale, () =>
        {
            throw new Error("No host lookup for a valid extension failure");
        }))
            .toBe(`${expected}\nEACCES: /Original/私密.txt`);
    }
    const before = structuredClone(retained);
    failure.error.messages.en = "Replacement package";
    expect(retained).toEqual(before);
    failure.error.messages.en = "Could not read private.txt";
});

test("host codes translate while local diagnostic text stays unchanged", () =>
{
    const error = operationFailure(new Error("transport original"), "textReadFailed");
    for (const locale of ["en", "zh-Hans"] as const)
    {
        const text = (value: LocalizedText) => resources.format(locale, value, "macos", 2048);
        const output = operationErrorText(error, locale, text);
        expect(output).toBe(`${text({ message: "error-operation-text-read-failed" })}\ntransport original`);
    }
    expect(operationFailure({ kind: "host", code: "textReadTimeout" }, "textReadFailed")).toEqual({
        kind: "host",
        code: "textReadTimeout"
    });
});

test("malformed and oversized presentations never expose raw message IDs or blank translations", () =>
{
    for (
        const corrupt of [
            (value: typeof failure) =>
            {
                delete (value.error.messages as Partial<typeof value.error.messages>)["zh-Hans"];
            },
            (value: typeof failure) =>
            {
                value.error.messages["zh-Hans"] = " ";
            },
            (value: typeof failure) =>
            {
                value.error.messages.en = "x".repeat(MAX_ERROR_MESSAGE_CHARS + 1);
            },
            (value: typeof failure) =>
            {
                value.error.error.detail = "界".repeat(MAX_ERROR_DETAIL_BYTES);
            },
            (value: typeof failure) =>
            {
                Object.assign(value.error.error.message, { scope: "host" });
            },
            (value: typeof failure) =>
            {
                Object.assign(value.error.error.message.args, { platform: "windows" });
            },
            (value: typeof failure) =>
            {
                value.error.error.code = "invalid code";
            },
            (value: typeof failure) =>
            {
                Object.assign(value.error.error, { payload: { ignoredButUnbounded: value } });
            }
        ]
    )
    {
        const invalid = structuredClone(failure);
        corrupt(invalid);
        const retained = operationFailure(invalid, "textReadFailed");
        expect(retained.kind).toBe("host");
        expect(operationErrorText(retained, "zh-Hans", value => resources.format("zh-Hans", value, "macos", 2048)))
            .toContain("扩展返回了无效响应");
    }
    expect(operationFailure("x".repeat(MAX_ERROR_DETAIL_BYTES + 1), "textReadFailed")).toMatchObject({
        code: "extensionProtocol"
    });
});

test("an unconfirmed action outcome never tells the user the action did not run", () =>
{
    const error = operationFailure(new Error("transport disconnected"), "invocationUnconfirmed");
    for (const [locale, expected] of [["en", "may have completed"], ["zh-Hans", "可能已经完成"]] as const)
    {
        for (const platform of ["macos", "windows"] as const)
        {
            const output = operationErrorText(error, locale, value => resources.format(locale, value, platform, 2048));
            expect(output).toContain(expected);
            expect(output).toContain("transport disconnected");
        }
    }
});

test("cleanup errors retain both failures, switch locales and reject recursive groups", () =>
{
    const combined = {
        kind: "cleanup",
        primary: { kind: "host", code: "navigationDepthReached", detail: "primary detail" },
        cleanup: structuredClone(failure)
    };
    const retained = operationFailure(combined, "viewOperationFailed");
    expect(retained).toEqual(combined);
    for (
        const [locale, heading, explanation] of [
            ["en", "Releasing the new view also failed:", "Too many views are open"],
            ["zh-Hans", "释放新视图也失败了：", "打开的视图过多"]
        ] as const
    )
    {
        const output = operationErrorText(retained, locale, value => resources.format(locale, value, "macos", 2048));
        expect(output).toContain(heading);
        expect(output).toContain(explanation);
        expect(output).toContain("primary detail");
        expect(output).toContain(failure.error.messages[locale]);
        expect(output).toContain(failure.error.error.detail);
    }
    for (
        const invalid of [
            { ...combined, primary: combined },
            { ...combined, cleanup: { kind: "cleanup" } },
            { ...combined, cleanup: null }
        ]
    )
    {
        expect(operationFailure(invalid, "viewOperationFailed")).toEqual({
            kind: "host",
            code: "extensionProtocol",
            detail: "Invalid structured error presentation."
        });
    }
    combined.cleanup.error.messages.en = "changed";
    expect(retained).not.toEqual(combined);
});
