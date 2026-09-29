import assert from "node:assert/strict";
import { setImmediate } from "node:timers/promises";
import { test } from "vitest";

import { SettingsSearchState } from "../../src/settings/SettingsSearchState.svelte.ts";

import type { SettingsSearchEntry } from "../../src/types/SettingsSearchEntry.ts";

const result: SettingsSearchEntry = {
    pageId: "general",
    target: { kind: "field", key: "theme" },
    title: "Theme",
    pageTitle: "General"
};

test("rapid input coalesces to the latest query and rejects obsolete deliveries", async () =>
{
    const calls: string[] = [];
    const deliveries: Array<(results: SettingsSearchEntry[]) => void> = [];
    const state = new SettingsSearchState(async query =>
    {
        calls.push(query);
        return new Promise(resolve =>
        {
            deliveries.push(resolve);
        });
    });
    state.update("t");
    state.update("th");
    state.update("theme");
    assert.deepEqual(calls, ["t"]);
    assert.equal(state.hasCompletedSearch, false);
    deliveries[0]?.([result]);
    await setImmediate();
    assert.deepEqual(calls, ["t", "theme"]);
    assert.deepEqual(state.results, []);
    deliveries[1]?.([result]);
    await setImmediate();
    assert.deepEqual(state.results, [result]);
    assert.equal(state.hasCompletedSearch, true);
    assert.equal(state.busy, false);
});

test("clearing and hiding suppress late results and hidden work", async () =>
{
    let complete!: (results: SettingsSearchEntry[]) => void;
    let calls = 0;
    const state = new SettingsSearchState(async () =>
    {
        calls++;
        return new Promise(resolve =>
        {
            complete = resolve;
        });
    });
    state.update("theme");
    state.update("");
    complete([result]);
    await setImmediate();
    assert.deepEqual(state.results, []);
    state.setVisible(false);
    state.update("theme");
    state.refresh();
    assert.equal(calls, 1);
    state.setVisible(true);
    assert.equal(calls, 2);
    state.setVisible(false);
    complete([result]);
    await setImmediate();
    assert.deepEqual(state.results, []);
    assert.equal(state.busy, false);
});

test("lifecycle refresh supersedes an in-flight query even when text is unchanged", async () =>
{
    const deliveries: Array<(results: SettingsSearchEntry[]) => void> = [];
    const state = new SettingsSearchState(async () =>
    {
        return new Promise(resolve =>
        {
            deliveries.push(resolve);
        });
    });
    state.update("theme");
    state.refresh();
    deliveries[0]?.([result]);
    await setImmediate();
    assert.deepEqual(state.results, []);
    assert.equal(deliveries.length, 2);
    deliveries[1]?.([]);
    await setImmediate();
    assert.deepEqual(state.results, []);
    assert.equal(state.busy, false);
});

test("current errors are visible, stale errors are ignored, and retries are explicit", async () =>
{
    let calls = 0;
    const state = new SettingsSearchState(async () =>
    {
        calls++;
        throw new Error("Search failed");
    });
    state.update("theme");
    await setImmediate();
    assert.match(state.error ?? "", /Search failed/);
    assert.equal(calls, 1);
    state.refresh();
    assert.equal(calls, 2);
    state.update("");
    await setImmediate();
    assert.equal(state.error, null);
    assert.equal(state.busy, false);
});

test("pending input retains the completed window until the latest ranking arrives", async () =>
{
    const deliveries: Array<(results: SettingsSearchEntry[]) => void> = [];
    const state = new SettingsSearchState(async () =>
    {
        return new Promise(resolve =>
        {
            deliveries.push(resolve);
        });
    });
    state.update("t");
    const initial = [result];
    deliveries[0]?.(initial);
    await setImmediate();
    assert.equal(state.hasCompletedSearch, true);
    state.update("th");
    state.update("theme");
    assert.equal(state.results, initial);
    assert.equal(state.hasCompletedSearch, true);
    assert.equal(state.busy, true);
    deliveries[1]?.([]);
    await setImmediate();
    assert.equal(state.results, initial);
    deliveries[2]?.([]);
    await setImmediate();
    assert.deepEqual(state.results, []);
    assert.equal(state.hasCompletedSearch, true);
    assert.equal(state.busy, false);
    state.update("no match");
    assert.equal(state.hasCompletedSearch, true);
    state.update("");
    assert.equal(state.hasCompletedSearch, false);
    deliveries[3]?.([result]);
    await setImmediate();
    assert.deepEqual(state.results, []);
    assert.equal(state.hasCompletedSearch, false);
});
