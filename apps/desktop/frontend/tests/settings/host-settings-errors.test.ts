import { expect, test } from "vitest";

import { hostResources } from "../../src/generated/host-resources";
import { ResourceSet } from "../../src/localization/ResourceSet";
import { settingsErrorText, settingsFailure } from "../../src/settings/error-presentation";
import { hostSettingsFailure } from "../../src/settings/host-settings-errors";
import { SettingsState } from "../../src/settings/SettingsState.svelte";

import type { HostSettingsFailure } from "../../src/generated/HostSettingsFailure";

test("host preference recovery retains all causes through locale changes without borrowing the receipt", () =>
{
    const source = _failure();
    const retained = settingsFailure(source);
    source.primary = { kind: "host", code: "cancelled" };
    const resources = new ResourceSet(hostResources);
    for (const locale of ["en", "zh-Hans"] as const)
    {
        const text = settingsErrorText(retained, locale, value => resources.format(locale, value, "macos", 2048));
        for (const detail of ["Original release", "Original rollback", "Original cleanup", "Original confirmation"])
        {
            expect(text).toContain(detail);
        }
        expect(text).toContain(locale === "en" ? "previous shortcut" : "原快捷键");
        expect(text).toContain(locale === "en" ? "Saved preferences could not be confirmed" : "无法确认已保存");
    }
});

test("missing recovery fields and nested failures cannot claim a valid host transaction", () =>
{
    const original = _failure();
    for (
        const invalid of [
            { kind: "HostSettingsFailure", primary: original.primary },
            { ...original, cleanup: { kind: "cleanup", primary: original.primary, cleanup: original.primary } },
            { ...original, confirmation: original }
        ]
    )
    {
        expect(hostSettingsFailure(invalid).primary).toMatchObject({ kind: "host", code: "extensionProtocol" });
    }
    expect(hostSettingsFailure(new Error("Original transport"))).toEqual({
        kind: "HostSettingsFailure",
        primary: { kind: "host", code: "preferencesUnconfirmed", detail: "Original transport" },
        rollback: null,
        cleanup: null,
        confirmation: null
    });
});

test("a host receipt reconciles saved facts while unconfirmed application requires an explicit edit", async () =>
{
    const original = { shortcut: "Ctrl+Space" };
    let writes = 0;
    const state = new SettingsState(
        { values: original, saved: original, effective: original, error: null },
        async ({ value }) =>
        {
            writes++;
            if (writes === 1)
            {
                // The new preference reached disk, but releasing and restoring shortcuts failed.
                return {
                    values: original,
                    saved: { shortcut: value },
                    effective: null,
                    error: { ..._failure(), confirmation: null }
                };
            }
            const next = { shortcut: value };
            return { values: next, saved: next, effective: next, error: null };
        }
    );
    await state.change("shortcut", "Ctrl+KeyK");
    expect(state.values.shortcut).toBe("Ctrl+Space");
    expect(state.saved.shortcut).toBe("Ctrl+KeyK");
    expect(state.effective).toBeNull();
    expect(state.errors.get("shortcut")).toEqual({ ..._failure(), confirmation: null });
    const resources = new ResourceSet(hostResources);
    for (const locale of ["en", "zh-Hans"] as const)
    {
        const error = state.errors.get("shortcut");
        if (!error)
        {
            throw new Error("Expected retained host failure");
        }
        settingsErrorText(error, locale, value => resources.format(locale, value, "macos", 2048));
    }
    expect(writes).toBe(1);
    await state.change("shortcut", "Ctrl+KeyK");
    expect(writes).toBe(2);
    expect(state.effective).toEqual({ shortcut: "Ctrl+KeyK" });
    expect(state.errors.size).toBe(0);
});

test("missing saved-state confirmation preserves the draft and last known saved value without retry", async () =>
{
    const original = { shortcut: "Ctrl+Space" };
    let writes = 0;
    const state = new SettingsState(
        { values: original, saved: original, effective: original, error: null },
        async () =>
        {
            writes++;
            throw Object.assign(new Error("Saved preferences could not be read"), _failure());
        }
    );
    await state.change("shortcut", "Ctrl+KeyK");
    expect(state.values.shortcut).toBe("Ctrl+KeyK");
    expect(state.saved.shortcut).toBe("Ctrl+Space");
    expect(state.effective).toBeNull();
    expect(state.errors.get("shortcut")).toEqual(_failure());
    state.commitEdits();
    await state.commit("shortcut");
    expect(writes).toBe(1);
});

function _failure(): HostSettingsFailure
{
    return {
        kind: "HostSettingsFailure",
        primary: { kind: "host", code: "shortcutReleaseFailed", detail: "Original release" },
        rollback: { kind: "host", code: "preferencesRollbackFailed", detail: "Original rollback" },
        cleanup: { kind: "host", code: "shortcutCleanupFailed", detail: "Original cleanup" },
        confirmation: { kind: "host", code: "preferencesConfirmationFailed", detail: "Original confirmation" }
    };
}
