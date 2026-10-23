import { readFileSync } from "node:fs";
import { runInNewContext } from "node:vm";
import { expect, test } from "vitest";

test("Isolation accepts every registered shell command and rejects unknown commands", () =>
{
    const shell = new URL("../../", import.meta.url);
    const source = readFileSync(new URL("isolation/index.js", shell), "utf8");
    const build = readFileSync(new URL("build.rs", shell), "utf8");
    const registered = build.match(/const COMMANDS:[^=]+=[\s\S]*?\[([\s\S]*?)\];/)?.[1];
    expect(registered).toBeDefined();
    const window: { __TAURI_ISOLATION_HOOK__?: (message: unknown) => unknown; } = {};
    runInNewContext(source, { window });
    const hook = window.__TAURI_ISOLATION_HOOK__;
    if (!hook)
    {
        throw new Error("Isolation hook was not installed");
    }
    const commands = [...(registered ?? "").matchAll(/"([a-z_]+)"/g)].map(match => match[1]);
    expect(commands).toContain("read_context_menu");
    const application = readFileSync(new URL("src/lib.rs", shell), "utf8");
    const handlers = application.match(/generate_handler!\[([\s\S]*?)\]/)?.[1];
    expect(handlers).toBeDefined();
    expect(new Set(handlers?.split(",").map(command => command.trim()).filter(Boolean))).toEqual(new Set(commands));
    for (const cmd of commands)
    {
        const message = { cmd, payload: {} };
        expect(hook(message)).toBe(message);
    }
    expect(() => hook({ cmd: "verification_report", payload: {} })).toThrow("Blocked IPC command");
    expect(() => hook({ cmd: "unknown", payload: {} })).toThrow("Blocked IPC command");
    expect(() => hook({ cmd: "read_context_menu", payload: null })).toThrow("Blocked malformed IPC payload");
});

test("both product windows can read localization through their session-authorized command", () =>
{
    const shell = new URL("../../", import.meta.url);
    for (const surface of ["launcher", "settings"])
    {
        const capability = JSON.parse(readFileSync(new URL(`capabilities/${surface}.json`, shell), "utf8")) as {
            permissions: string[];
            windows: string[];
        };
        expect(capability.windows).toEqual([surface]);
        expect(capability.permissions).toContain("allow-read-localization-resources");
    }
});

test("only the launcher can request its retained startup diagnostic", () =>
{
    const shell = new URL("../../", import.meta.url);
    for (const surface of ["launcher", "settings"])
    {
        const capability = JSON.parse(readFileSync(new URL(`capabilities/${surface}.json`, shell), "utf8")) as {
            permissions: string[];
        };
        expect(capability.permissions.includes("allow-read-startup-diagnostic")).toBe(surface === "launcher");
    }
});
