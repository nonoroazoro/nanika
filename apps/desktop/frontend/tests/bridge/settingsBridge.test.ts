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
            { key: "hideOnBlur", value: true }
        ] satisfies HostSettingsChange[]
    )
    {
        await settingsBridge.saveHost(request);
        expect(harness.invoke).toHaveBeenLastCalledWith("save_host_settings", { request });
    }
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
