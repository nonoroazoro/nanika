import assert from "node:assert/strict";
import { test } from "vitest";

import { StartupSettings } from "../../src/settings/StartupSettings.svelte.ts";

import type { StartupStatus } from "../../src/types/Settings.ts";

test("activation refresh observes OS changes and shares concurrent reads", async () =>
{
    let status: StartupStatus = "requiresApproval";
    let reads = 0;
    const startup = new StartupSettings(
        async () =>
        {
            reads++;
            return status;
        },
        async () => status,
        () =>
        {}
    );
    await startup.refresh();
    assert.equal(startup.status, "requiresApproval");
    status = "enabled";
    await Promise.all([startup.refresh(), startup.refresh()]);
    assert.equal(reads, 2);
    assert.equal(startup.status, "enabled");
    assert.equal(startup.settings.values.enabled, true);
});

test("focus return during native write reads only after the operation settles", async () =>
{
    let resolve!: (status: StartupStatus) => void;
    const pending = new Promise<StartupStatus>(complete =>
    {
        resolve = complete;
    });
    let status: StartupStatus = "disabled";
    const events: string[] = [];
    const startup = new StartupSettings(async () =>
    {
        events.push("read");
        return status;
    }, async () =>
    {
        events.push("write");
        status = await pending;
        return status;
    }, () =>
    {});
    await startup.refresh();
    const write = startup.settings.change("enabled", true);
    const refresh = startup.refresh();
    await Promise.resolve();
    assert.deepEqual(events, ["read", "write"]);
    resolve("enabled");
    await Promise.all([write, refresh]);
    assert.deepEqual(events, ["read", "write", "read"]);
    assert.equal(startup.settings.values.enabled, true);
});

test("a failed read can recover on the next explicit activation without poisoning writes", async () =>
{
    let fail = true;
    const startup = new StartupSettings(
        async () =>
        {
            if (fail)
            {
                throw new Error("OS unavailable");
            }
            return "disabled";
        },
        async () => "enabled",
        () =>
        {}
    );
    await assert.rejects(startup.refresh(), /OS unavailable/);
    assert.equal(startup.status, null);
    fail = false;
    await startup.refresh();
    await startup.settings.change("enabled", true);
    assert.equal(startup.settings.values.enabled, true);
});
