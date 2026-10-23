import assert from "node:assert/strict";
import { test } from "vitest";

import { configurationProperty } from "../../fixtures/configuration";
import { ExtensionSettingsState } from "../../src/settings/ExtensionSettingsState.svelte";

import type { ConfigurationSaveOutcome } from "../../src/generated/ConfigurationSaveOutcome";
import type { ExtensionSettings } from "../../src/generated/ExtensionSettings";
import type { SettingsError } from "../../src/settings/SettingsError";

function _configuration(revision: number, enabled: boolean): NonNullable<ExtensionSettings["configuration"]>
{
    return {
        revision,
        extensionId: "test",
        values: { enabled, title: "Saved" },
        saved: { enabled, title: "Saved" },
        effective: { enabled, title: "Saved" },
        contribution: {
            actions: [],
            quickMenu: [],
            title: "Test",
            properties: {
                enabled: configurationProperty({
                    type: "boolean",
                    title: "Enabled",
                    description: null,
                    default: false,
                    properties: {},
                    required: [],
                    persistence: "beforeApply"
                }),
                title: configurationProperty({
                    type: "string",
                    title: "Title",
                    description: null,
                    default: "Saved",
                    properties: {},
                    required: [],
                    persistence: "beforeApply",
                    maxLength: 100
                })
            }
        }
    };
}

function _pending(): {
    promise: Promise<ConfigurationSaveOutcome>;
    resolve: (result: ConfigurationSaveOutcome) => void;
}
{
    let resolve!: (result: ConfigurationSaveOutcome) => void;
    const promise = new Promise<ConfigurationSaveOutcome>(complete =>
    {
        resolve = complete;
    });
    return { promise, resolve };
}

test("a lifecycle snapshot queued during a save cannot undo the completed write", async () =>
{
    const pending = _pending();
    const state = new ExtensionSettingsState(_configuration(1, false), null, async () => pending.promise, () =>
    {});
    const changed = state.change("enabled", true);
    state.observeConfiguration(_configuration(1, false));
    pending.resolve({ ..._configuration(2, true), error: null });
    await changed;
    await new Promise(resolve =>
    {
        setTimeout(resolve, 0);
    });
    assert.equal(state.values.enabled, true);
    assert.equal(state.saved.enabled, true);
    state.observeConfiguration(_configuration(1, false));
    await Promise.resolve();
    assert.equal(state.values.enabled, true);
});

test("newer lifecycle facts survive a late failed completion and preserve unsaved drafts", async () =>
{
    const pending = _pending();
    const errors: SettingsError[] = [];
    const state = new ExtensionSettingsState(
        _configuration(1, false),
        null,
        async () => pending.promise,
        error =>
        {
            errors.push(error);
        }
    );
    const changed = state.change("enabled", true);
    state.edit("title", "My draft");
    state.observeConfiguration(_configuration(3, false));
    pending.resolve({
        ..._configuration(2, true),
        error: {
            code: "savedButNotApplied",
            cause: { kind: "host", code: "extensionIo", detail: "device rejected request" }
        }
    });
    await changed;
    await Promise.resolve();
    assert.equal(state.values.enabled, false);
    assert.equal(state.saved.enabled, false);
    assert.equal(state.values.title, "My draft");
    assert.deepEqual(errors, [{
        code: "savedButNotApplied",
        cause: { kind: "host", code: "extensionIo", detail: "device rejected request" }
    }]);
});

test("resumed operations share the same ordering as new writes", async () =>
{
    const pending = _pending();
    const state = new ExtensionSettingsState(_configuration(1, false), null, async () => pending.promise, () =>
    {});
    state.resumeConfiguration("enabled", pending.promise);
    state.observeConfiguration(_configuration(1, false));
    pending.resolve({ ..._configuration(2, true), error: null });
    await new Promise(resolve =>
    {
        setTimeout(resolve, 0);
    });
    assert.equal(state.values.enabled, true);
    assert.equal(state.phase.size, 0);
});

test("rejected admission preserves the draft and facts without implicit retries", async () =>
{
    const notifications: SettingsError[] = [];
    let writes = 0;
    const failure = { code: "rejected", cause: { kind: "host", code: "operationPending" } } as const;
    const state = new ExtensionSettingsState(_configuration(1, false), null, async () =>
    {
        if (++writes === 1)
        {
            throw Object.assign(new Error("rejected"), failure);
        }
        return { ..._configuration(3, true), error: null };
    }, error =>
    {
        notifications.push(error);
    });
    await state.change("enabled", true);
    assert.equal(state.values.enabled, true);
    assert.equal(state.saved.enabled, false);
    assert.equal(state.effective?.enabled, false);
    assert.deepEqual(state.errors.get("enabled"), failure);
    state.observeConfiguration(_configuration(2, false));
    state.commitEdits();
    await Promise.resolve();
    await Promise.resolve();
    assert.equal(state.values.enabled, true);
    assert.equal(writes, 1);
    assert.deepEqual(notifications, [failure]);
    await state.change("enabled", true);
    assert.equal(writes, 2);
    assert.equal(state.saved.enabled, true);
    assert.equal(state.effective?.enabled, true);
    assert.equal(state.errors.has("enabled"), false);
});

test("lost configuration completion retains input and reports unknown effective state", async () =>
{
    const state = new ExtensionSettingsState(_configuration(1, false), null, async () =>
    {
        throw new Error("Original transport detail");
    }, () =>
    {});
    await state.change("title", "Unsaved input");
    assert.equal(state.values.title, "Unsaved input");
    assert.equal(state.saved.title, "Saved");
    assert.equal(state.effective, null);
    assert.deepEqual(state.errors.get("title"), {
        code: "unconfirmed",
        cause: { kind: "host", code: "configurationUnconfirmed", detail: "Original transport detail" }
    });
    state.commitEdits();
    assert.equal(state.phase.size, 0);
});
