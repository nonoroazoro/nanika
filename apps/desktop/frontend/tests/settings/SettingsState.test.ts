import assert from "node:assert/strict";
import { test } from "vitest";

import { SettingsState } from "../../src/settings/SettingsState.svelte.ts";
import { prepareSetting } from "../../src/settings/validation.ts";

import type { SettingsWriteResult } from "../../src/settings/SettingsWriteResult.ts";
import type { ConfigurationSchema, ConfigurationValue } from "../../src/types/Settings.ts";

function _result<T>(values: T): SettingsWriteResult<T>
{
    return { values, saved: values, effective: values, error: null };
}

function _gate<T>(): { promise: Promise<T>; resolve: (value: T) => void; }
{
    let resolve!: (value: T) => void;
    const promise = new Promise<T>(complete =>
    {
        resolve = complete;
    });
    return { promise, resolve };
}

test("one operation per field, isolated patches, and independent editing", async () =>
{
    const gate = _gate<undefined>();
    let saved = { enabled: false, title: "Before", nested: { limit: 10 } };
    const patches: unknown[] = [];
    const state = new SettingsState(_result(saved), async (key, value) =>
    {
        patches.push({ key, value });
        await gate.promise;
        saved = { ...saved, [key]: value };
        return _result(saved);
    });
    const untouched = state.values.nested;
    const first = state.change("enabled", true);
    await state.change("enabled", false);
    state.edit("title", "After");
    assert.equal(state.phase.get("title"), undefined);
    assert.equal(state.values.title, "After");
    const second = state.commit("title");
    assert.equal(state.phase.get("title"), "queued");
    assert.equal(patches.length, 1);
    gate.resolve(undefined);
    await Promise.all([first, second]);
    assert.deepEqual(patches, [{ key: "enabled", value: true }, { key: "title", value: "After" }]);
    assert.equal(state.values.nested, untouched, "unrelated reactive object identity remains stable");
    assert.equal(state.values.enabled, true);
});

test("terminal backend facts determine saved and effective state independently", async () =>
{
    for (
        const result of [
            {
                values: { enabled: true },
                saved: { enabled: true },
                effective: null,
                error: "Saved, application failed"
            },
            { values: { enabled: false }, saved: { enabled: false }, effective: null, error: "Application rejected" },
            {
                values: { enabled: true },
                saved: { enabled: false },
                effective: { enabled: true },
                error: "Applied, persistence failed"
            }
        ]
    )
    {
        const gate = _gate<SettingsWriteResult<{ enabled: boolean; }>>();
        const notifications: string[] = [];
        const state = new SettingsState(_result({ enabled: false }), async () => gate.promise, undefined, error =>
        {
            notifications.push(error);
        });
        const operation = state.change("enabled", true);
        await Promise.resolve();
        assert.equal(state.phase.get("enabled"), "running");
        assert.equal(state.saved.enabled, false, "submission does not imply persistence");
        gate.resolve(result);
        await operation;
        assert.equal(state.phase.get("enabled"), undefined);
        assert.deepEqual(state.saved, result.saved);
        assert.deepEqual(state.values, result.values);
        assert.deepEqual(state.effective, result.effective);
        assert.deepEqual(notifications, [result.error]);
    }
});

test("invalid edits never submit or retry on navigation", async () =>
{
    let writes = 0;
    const state = new SettingsState(_result({ limit: 1 }), async () =>
    {
        writes++;
        return _result({ limit: 1 });
    }, (_key, value) => (value === 0 ? { error: "Enter 1 or more." } : { value }));
    await state.change("limit", 0);
    state.commitEdits();
    await Promise.resolve();
    assert.equal(writes, 0);
    assert.equal(state.values.limit, 0);
    assert.equal(state.saved.limit, 1);
});

test("reopened operation blocks repeated activation and serializes later edits", async () =>
{
    const gate = _gate<SettingsWriteResult<{ enabled: boolean; title: string; }>>();
    const writes: unknown[] = [];
    const state = new SettingsState(_result({ enabled: false, title: "Before" }), async (key, value) =>
    {
        writes.push({ key, value });
        return _result({ enabled: true, title: "After" });
    });
    state.resume("enabled", gate.promise);
    await state.change("enabled", false);
    const second = state.change("title", "After");
    assert.equal(writes.length, 0);
    gate.resolve(_result({ enabled: true, title: "Before" }));
    await second;
    assert.deepEqual(writes, [{ key: "title", value: "After" }]);
    assert.equal(state.values.enabled, true);
});

test("transport failure clears pending, reports uncertainty, and leaves later edits usable", async () =>
{
    const state = new SettingsState(_result({ enabled: false }), async () =>
    {
        throw new Error("Disconnected");
    });
    await state.change("enabled", true);
    assert.equal(state.phase.get("enabled"), undefined);
    assert.equal(state.effective, null);
    assert.match(state.errors.get("enabled") ?? "", /Disconnected/);
});

test("afterApply storage failure can be reversed to the saved value", async () =>
{
    let writes = 0;
    const state = new SettingsState(_result({ enabled: false }), async (_key, enabled) =>
    {
        writes++;
        return writes === 1
            ? { values: { enabled: true }, saved: { enabled: false }, effective: { enabled: true }, error: "Disk full" }
            : _result({ enabled });
    });
    await state.change("enabled", true);
    await state.change("enabled", false);
    assert.equal(writes, 2);
    assert.equal(state.effective?.enabled, false);
});

test("an explicit unchanged edit retries failed application, but navigation does not", async () =>
{
    let writes = 0;
    const state = new SettingsState(_result({ enabled: false }), async (_key, enabled) =>
    {
        writes++;
        return { ..._result({ enabled }), error: writes === 1 ? "Application failed" : null };
    });
    await state.change("enabled", true);
    state.commitEdits();
    await state.commit("enabled");
    assert.equal(writes, 1);
    await state.change("enabled", true);
    assert.equal(writes, 2);
    await state.change("enabled", true);
    assert.equal(writes, 2, "settled equal values do not repeat domain work");
});

test("unrelated completion preserves invalid drafts without retrying their validation", async () =>
{
    const notifications: string[] = [];
    const state = new SettingsState(
        _result({ limit: 10, enabled: false }),
        async () => _result({ limit: 10, enabled: true }),
        (key, value) => (key === "limit" && value === 0 ? { error: "Minimum is 1" } : { value }),
        error =>
        {
            notifications.push(error);
        }
    );
    await state.change("limit", 0);
    await state.change("enabled", true);
    state.commitEdits();
    await state.commit("limit");
    assert.equal(state.values.limit, 0);
    assert.equal(state.saved.limit, 10);
    assert.deepEqual(notifications, ["Minimum is 1"]);
});

test("schema-valid prototype names remain editable and field-local", async () =>
{
    let saved = { constructor: false, toString: false };
    let writes = 0;
    const state = new SettingsState(_result(saved), async (key, value) =>
    {
        writes++;
        saved = { ...saved, [key]: value };
        return _result(saved);
    });
    assert.equal(state.phase.get("constructor"), undefined);
    assert.equal(state.errors.get("toString"), undefined);
    await state.change("constructor", true);
    await state.change("toString", true);
    assert.equal(writes, 2);
    assert.deepEqual(state.values, { constructor: true, toString: true });
});

test("external refresh preserves newer edits and serializes their write", async () =>
{
    const gate = _gate<SettingsWriteResult<{ enabled: boolean; }>>();
    const writes: boolean[] = [];
    const state = new SettingsState(_result({ enabled: false }), async (_key, enabled) =>
    {
        writes.push(enabled);
        return _result({ enabled });
    });
    const refresh = state.refresh(async () => gate.promise);
    const edit = state.change("enabled", false);
    assert.equal(state.values.enabled, false);
    assert.deepEqual(writes, []);
    gate.resolve(_result({ enabled: true }));
    await Promise.all([refresh, edit]);
    assert.deepEqual(writes, [false]);
    assert.equal(state.values.enabled, false);
    assert.equal(state.effective?.enabled, false);
});

const integer: ConfigurationSchema = {
    type: "integer",
    minimum: 1,
    maximum: 5000,
    multipleOf: 1,
    allowUnlimited: true,
    properties: {},
    required: []
};
const text: ConfigurationSchema = { type: "string", maxLength: 100, properties: {}, required: [] };

test("equivalent numeric drafts never enter the operation queue, including nested values", async () =>
{
    const nested: ConfigurationSchema = {
        type: "array",
        maxItems: 10,
        properties: {},
        required: [],
        items: { type: "object", properties: { limit: integer, name: text }, required: ["limit", "name"] }
    };
    const cases: Array<{ draft: ConfigurationValue; saved: ConfigurationValue; schema: ConfigurationSchema; }> = [
        { schema: integer, saved: 50, draft: "50" },
        { schema: integer, saved: 50, draft: "0050" },
        { schema: integer, saved: null, draft: null },
        { schema: nested, saved: [{ limit: 50, name: "001" }], draft: [{ limit: "0050", name: "001" }] }
    ];
    for (const { schema, saved, draft } of cases)
    {
        const initial = { setting: saved, untouched: { title: "Keep" } };
        let writes = 0;
        const state = new SettingsState<Record<string, ConfigurationValue>>(_result(initial), async () =>
        {
            writes++;
            return _result(initial);
        }, (key, value) => prepareSetting({ setting: schema }, key, value));
        const untouched = state.values.untouched;
        state.edit("setting", draft);
        const commit = state.commit("setting");
        assert.equal(state.phase.has("setting"), false, "no pending state for a semantic no-op");
        assert.equal(state.startedAt.has("setting"), false);
        await commit;
        assert.equal(writes, 0);
        assert.deepEqual(state.values.setting, saved);
        assert.equal(state.values.untouched, untouched);
    }
});

test("writers receive the prepared value while string settings retain their type", async () =>
{
    let saved: Record<string, ConfigurationValue> = { limit: 50, name: "001" };
    const writes: Array<{ key: string; value: ConfigurationValue; }> = [];
    const state = new SettingsState(_result(saved), async (key, value) =>
    {
        writes.push({ key, value });
        saved = { ...saved, [key]: value };
        return _result(saved);
    }, (key, value) => prepareSetting({ limit: integer, name: text }, key, value));
    await state.change("limit", "0051");
    await state.change("name", "1");
    assert.deepEqual(writes, [{ key: "limit", value: 51 }, { key: "name", value: "1" }]);
});

test("preparation rejects incomplete numeric drafts without changing their text or admitting work", async () =>
{
    let writes = 0;
    const state = new SettingsState<Record<string, ConfigurationValue>>(_result({ limit: 50 }), async () =>
    {
        writes++;
        return _result({ limit: 50 });
    }, (key, value) => prepareSetting({ limit: integer }, key, value));
    for (const draft of ["", "-", "1e2", "1.5"])
    {
        await state.change("limit", draft);
        assert.equal(state.values.limit, draft);
        assert.equal(state.phase.has("limit"), false);
        assert.equal(state.errors.has("limit"), true);
    }
    assert.equal(writes, 0);
    await state.change("limit", "0050");
    assert.equal(writes, 0);
    assert.equal(state.errors.has("limit"), false);
    assert.equal(state.values.limit, 50);
});

test("canonical equality does not suppress an explicit retry after failed application", async () =>
{
    let writes = 0;
    const state = new SettingsState<Record<string, ConfigurationValue>>(_result({ limit: 50 }), async (_key, value) =>
    {
        writes++;
        return { ..._result({ limit: value }), error: writes === 1 ? "Application failed" : null };
    }, (key, value) => prepareSetting({ limit: integer }, key, value));
    await state.change("limit", "51");
    state.commitEdits();
    assert.equal(writes, 1);
    await state.change("limit", "0051");
    assert.equal(writes, 2);
    await state.change("limit", "51");
    assert.equal(writes, 2);
});
