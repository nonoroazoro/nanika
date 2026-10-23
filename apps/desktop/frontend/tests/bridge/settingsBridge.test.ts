import { Channel } from "@tauri-apps/api/core";
import { beforeEach, expect, expectTypeOf, test, vi } from "vitest";

import { settingsBridge } from "../../src/bridge/settingsBridge";

import type { HostSettingsChange } from "../../src/generated/HostSettingsChange";
import type { SaveSettingsRequest } from "../../src/generated/SaveSettingsRequest";
import type { LauncherSettings } from "../../src/settings/LauncherSettings";
import type { SettingsChange } from "../../src/settings/SettingsChange";

const harness = vi.hoisted(() => ({ invoke: vi.fn().mockResolvedValue(undefined) }));

vi.mock("@tauri-apps/api/core", () => ({ invoke: harness.invoke, Channel: vi.fn() }));

beforeEach(() =>
{
    harness.invoke.mockClear();
});

test("Settings resources carry session and package authority without paths", async () =>
{
    const request = {
        surface: "settings" as const,
        sessionId: 7,
        scope: { extensionId: "com.example.extension", identity: "a".repeat(64) }
    };
    await settingsBridge.readResources(request);
    expect(harness.invoke).toHaveBeenLastCalledWith("read_localization_resources", { request });
    expectTypeOf<{ instanceId: number; scope: typeof request.scope; sessionId: number; surface: "launcher"; }>()
        .not.toExtend<Parameters<typeof settingsBridge.readResources>[0]>();
});

test("launcher edits match the Rust write contract and exclude storage metadata", async () =>
{
    expectTypeOf<SettingsChange<LauncherSettings>>().toEqualTypeOf<HostSettingsChange>();
    expectTypeOf<Parameters<typeof settingsBridge.saveHost>>().toEqualTypeOf<[HostSettingsChange]>();
    expectTypeOf<{ key: "formatVersion"; value: 1; }>().not.toExtend<HostSettingsChange>();
    expectTypeOf<{ key: "hideOnBlur"; value: "dark"; }>().not.toExtend<HostSettingsChange>();
    for (
        const request of [
            { key: "launcherShortcut", value: "Alt+Space" },
            { key: "theme", value: "dark" },
            { key: "language", value: "zh-Hans" },
            { key: "hideOnBlur", value: true }
        ] satisfies HostSettingsChange[]
    )
    {
        const preferences = {
            formatVersion: 1,
            launcherShortcut: "Ctrl+Space",
            theme: "system",
            language: "system",
            hideOnBlur: true
        };
        harness.invoke.mockResolvedValueOnce({
            values: preferences,
            saved: preferences,
            effective: preferences,
            error: null
        });
        await settingsBridge.saveHost(request);
        expect(harness.invoke).toHaveBeenLastCalledWith("save_host_settings", { request });
    }
});

test("progress receipts wait for presentation while terminal results bypass preparation", async () =>
{
    const preparation = Promise.withResolvers<undefined>();
    const observe = vi.fn(async (update: { result: { status: string; }; }): Promise<void> =>
    {
        if (update.result.status === "running")
        {
            await preparation.promise;
        }
    });
    const error = vi.fn<(error: unknown) => void>();
    const ignore = (): void =>
    {};
    await settingsBridge.read(observe, ignore, ignore, error, ignore, ignore, ignore);
    const channel = vi.mocked(Channel).mock.instances[0];
    if (!channel)
    {
        throw new Error("Expected Settings channel");
    }
    harness.invoke.mockClear();
    channel.onmessage({
        type: "application",
        deliveryId: 7,
        update: { requestId: 1, extensionId: "example", key: "scan", result: { status: "running", progress: null } }
    });
    channel.onmessage({
        type: "application",
        update: {
            requestId: 1,
            extensionId: "example",
            key: "scan",
            result: {
                status: "failed",
                error: {
                    code: "unconfirmed",
                    cause: { kind: "host", code: "configurationUnconfirmed", detail: "Diagnostic" }
                }
            }
        }
    });
    expect(observe).toHaveBeenCalledTimes(2);
    await Promise.resolve();
    expect(harness.invoke).not.toHaveBeenCalled();
    preparation.resolve(undefined);
    await vi.waitFor(() =>
    {
        expect(harness.invoke).toHaveBeenCalledExactlyOnceWith("acknowledge_settings_delivery", { deliveryId: 7 });
    });
    expect(error).not.toHaveBeenCalled();
});

test("host save transport failures keep an unconfirmed outcome with the original diagnostic", async () =>
{
    harness.invoke.mockRejectedValueOnce(new Error("Channel closed after submission"));
    await expect(settingsBridge.saveHost({ key: "language", value: "zh-Hans" })).rejects.toMatchObject({
        kind: "HostSettingsFailure",
        primary: { kind: "host", code: "preferencesUnconfirmed", detail: "Channel closed after submission" },
        rollback: null,
        cleanup: null,
        confirmation: null
    });
    expect(harness.invoke).toHaveBeenCalledTimes(1);
});

test("extension settings forward the generated request without rebuilding its fields", async () =>
{
    expectTypeOf<Parameters<typeof settingsBridge.save>>().toEqualTypeOf<[SaveSettingsRequest]>();
    const request: SaveSettingsRequest = {
        extensionId: "test.extension",
        key: "options",
        value: { nested: [null, true, 1, "text"] }
    };
    await settingsBridge.save(request);
    expect(harness.invoke).toHaveBeenLastCalledWith("save_settings", { request });
});
