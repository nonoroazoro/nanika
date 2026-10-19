import { readFileSync } from "node:fs";
import { runInNewContext } from "node:vm";
import { expect, test } from "vitest";

test("Verification isolation accepts only its registered commands", () =>
{
    const root = new URL("../../", import.meta.url);
    const source = readFileSync(new URL("isolation/index.js", root), "utf8");
    const build = readFileSync(new URL("build.rs", root), "utf8");
    const commands = [...build.matchAll(/"(verification_[a-z_]+)"/g)].map(match => match[1]);
    expect(commands.length).toBeGreaterThan(0);
    const window: { __TAURI_ISOLATION_HOOK__?: (message: unknown) => unknown; } = {};
    runInNewContext(source, { window });
    const hook = window.__TAURI_ISOLATION_HOOK__;
    if (!hook)
    {
        throw new Error("Isolation hook was not installed");
    }
    for (const cmd of commands)
    {
        const message = { cmd, payload: {} };
        expect(hook(message)).toBe(message);
        expect(() => hook({ cmd, payload: null })).toThrow("Blocked verification IPC");
    }
    for (const cmd of ["invoke_candidate", "invoke_settings_action", "unknown"])
    {
        expect(() => hook({ cmd, payload: {} })).toThrow("Blocked verification IPC");
    }
});
