import assert from "node:assert/strict";
import { test } from "vitest";

import { RangeReadState } from "../../src/ui/RangeReadState.svelte";

test("failed reads remain idle until explicit retry, then the same range can succeed", async () =>
{
    const state = new RangeReadState();
    let calls = 0;
    const load = async (): Promise<number | null> => (++calls === 1 ? null : 2);
    await state.read("collection:0:30", load);
    assert.equal(state.failed, true);
    await state.read("collection:0:30", load);
    assert.equal(calls, 1);
    state.reset();
    await state.read("collection:0:30", load);
    assert.equal(state.failed, false);
    assert.equal(calls, 2);
    await state.read("collection:0:30", load);
    assert.equal(calls, 2);
});

test("in-flight reads are bounded and obsolete completions cannot overwrite a new scope", async () =>
{
    const state = new RangeReadState();
    let finish!: (revision: number | null) => void;
    const pending = state.read("old", async () =>
        new Promise(resolve =>
        {
            finish = resolve;
        }));
    await state.read("other", async () =>
    {
        assert.fail("must not send a second read while the first is pending");
    });
    state.reset();
    await state.read("new", async () => 3);
    finish(null);
    await pending;
    assert.equal(state.failed, false);
});

test("a rejected transport is recoverable and another viewport range remains eligible", async () =>
{
    const state = new RangeReadState();
    await state.read("first", async () =>
    {
        throw new Error("Disconnected");
    });
    assert.equal(state.failed, true);
    await state.read("second", async () => 4);
    assert.equal(state.failed, false);
});
