import assert from "node:assert/strict";
import { test } from "vitest";

import { operationFailure } from "../../src/localization/operation-errors";
import { StartupSettings } from "../../src/settings/StartupSettings.svelte.ts";

import type { StartupStatus } from "../../src/generated/StartupStatus";

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
    await assert.rejects(startup.refresh(), error =>
    {
        assert.deepEqual(operationFailure(error, "startupReadFailed"), {
            kind: "host",
            code: "startupReadFailed",
            detail: "OS unavailable"
        });
        return true;
    });
    assert.deepEqual(startup.readError, { kind: "host", code: "startupReadFailed", detail: "OS unavailable" });
    assert.equal(startup.reading, false);
    assert.equal(startup.status, null);
    fail = false;
    await startup.refresh();
    assert.equal(startup.readError, null);
    await startup.settings.change("enabled", true);
    assert.equal(startup.settings.values.enabled, true);
});

test("a failed startup write retains its code and uses the confirmed status without retrying", async () =>
{
    const failure = {
        kind: "host",
        code: "startupWriteUnconfirmed",
        detail: "SMAppService original diagnostic"
    } as const;
    const notifications: unknown[] = [];
    let reads = 0;
    let writes = 0;
    const startup = new StartupSettings(async () =>
    {
        reads++;
        return "disabled";
    }, async () =>
    {
        writes++;
        throw Object.assign(new Error("Fixture startup failure"), failure);
    }, error =>
    {
        notifications.push(error);
    });
    await startup.refresh();
    await startup.settings.change("enabled", true);
    assert.equal(reads, 2);
    assert.equal(writes, 1);
    assert.equal(startup.status, "disabled");
    assert.equal(startup.readError, null);
    assert.deepEqual(startup.settings.effective, { enabled: false });
    assert.deepEqual(startup.settings.errors.get("enabled"), failure);
    assert.deepEqual(notifications, [failure]);
});

test("failed status recovery preserves both causes and the draft until an explicit retry", async () =>
{
    const writeFailure = { kind: "host", code: "startupWriteUnconfirmed", detail: "Original write rejection" } as const;
    const readFailure = { kind: "host", code: "startupReadFailed", detail: "Original status rejection" } as const;
    let reject = false;
    let writes = 0;
    const startup = new StartupSettings(async () =>
    {
        if (reject)
        {
            throw Object.assign(new Error("Fixture startup failure"), readFailure);
        }
        return "disabled";
    }, async () =>
    {
        writes++;
        if (reject)
        {
            throw Object.assign(new Error("Fixture startup failure"), writeFailure);
        }
        return "enabled";
    }, () =>
    {});
    await startup.refresh();
    reject = true;
    await startup.settings.change("enabled", true);
    assert.equal(writes, 1);
    assert.equal(startup.settings.values.enabled, true);
    assert.deepEqual(startup.settings.saved, { enabled: false });
    assert.equal(startup.settings.effective, null);
    assert.deepEqual(startup.settings.errors.get("enabled"), writeFailure);
    assert.deepEqual(startup.readError, readFailure);
    reject = false;
    await startup.refresh();
    assert.equal(writes, 1);
    assert.equal(startup.readError, null);
    assert.equal(startup.settings.values.enabled, true);
    await startup.settings.change("enabled", true);
    assert.equal(writes, 2);
    assert.deepEqual(startup.settings.effective, { enabled: true });
});
