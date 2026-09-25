import assert from "node:assert/strict";
import { test } from "vitest";

import { ExtensionSettingsState } from "../../src/settings/ExtensionSettingsState.svelte";

import type { ConfigurationWriteResult } from "../../src/settings/ConfigurationWriteResult";
import type { ExtensionSettings } from "../../src/types/Settings";

function _configuration(revision: number, enabled: boolean): NonNullable<ExtensionSettings["configuration"]>
{
    return {
        revision,
        extensionId: "test",
        values: { enabled, title: "Saved" },
        saved: { enabled, title: "Saved" },
        effective: { enabled, title: "Saved" },
        contribution: {
            title: "Test",
            properties: {
                enabled: {
                    type: "boolean",
                    title: "Enabled",
                    description: null,
                    default: false,
                    properties: {},
                    required: [],
                    persistence: "beforeApply"
                },
                title: {
                    type: "string",
                    title: "Title",
                    description: null,
                    default: "Saved",
                    properties: {},
                    required: [],
                    persistence: "beforeApply",
                    maxLength: 100
                }
            }
        }
    };
}

function _pending(): {
    promise: Promise<ConfigurationWriteResult>;
    resolve: (result: ConfigurationWriteResult) => void;
}
{
    let resolve!: (result: ConfigurationWriteResult) => void;
    const promise = new Promise<ConfigurationWriteResult>(complete =>
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
    const errors: string[] = [];
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
    pending.resolve({ ..._configuration(2, true), error: "Saved but application failed" });
    await changed;
    await Promise.resolve();
    assert.equal(state.values.enabled, false);
    assert.equal(state.saved.enabled, false);
    assert.equal(state.values.title, "My draft");
    assert.deepEqual(errors, ["Saved but application failed"]);
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
