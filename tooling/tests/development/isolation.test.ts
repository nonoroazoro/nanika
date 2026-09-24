import { readFileSync } from "node:fs";
import { runInNewContext } from "node:vm";
import { expect, test } from "vitest";

test("Isolation accepts every registered shell command and rejects unknown commands", () =>
{
    const shell = new URL("../../../apps/desktop/shell/", import.meta.url);
    const source = readFileSync(new URL("isolation/index.js", shell), "utf8");
    const build = readFileSync(new URL("build.rs", shell), "utf8");
    const registered = build.slice(build.indexOf("= &["), build.indexOf("];"));
    const window: { __TAURI_ISOLATION_HOOK__?: (message: unknown) => unknown; } = {};
    runInNewContext(source, { window });
    const hook = window.__TAURI_ISOLATION_HOOK__;
    if (!hook)
    {
        throw new Error("Isolation hook was not installed");
    }
    const commands = [...registered.matchAll(/"([a-z_]+)"/g)].map(match => match[1]);
    expect(commands).toContain("read_context_menu");
    for (const cmd of commands)
    {
        const message = { cmd, payload: {} };
        expect(hook(message)).toBe(message);
    }
    expect(() => hook({ cmd: "unknown", payload: {} })).toThrow("Blocked IPC command");
    expect(() => hook({ cmd: "read_context_menu", payload: null })).toThrow("Blocked malformed IPC payload");
});
