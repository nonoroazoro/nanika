import { appendFile, mkdir } from "node:fs/promises";
import { join } from "node:path";
import { createInterface } from "node:readline";
import { Readable } from "node:stream";
import { fileURLToPath } from "node:url";
import { remote } from "webdriverio";

import { NativeFixture } from "./NativeFixture.ts";
import { OwnedProcess } from "../../quality/OwnedProcess.ts";
import { processRunner, stopProcessTree } from "../../quality/process-tree.ts";

import type { NativeState } from "./NativeState.ts";

const scenario = process.argv[2] ?? "application";
if (!["application", "activation", "ime"].includes(scenario))
{
    throw new Error("Expected application, activation or ime scenario");
}
const cancellation = new AbortController();
const interrupt = () =>
{
    cancellation.abort(new Error("E2E interrupted"));
};
process.on("SIGINT", interrupt);
process.on("SIGTERM", interrupt);
const control = createInterface({ input: process.stdin });
control.on("line", value =>
{
    if (value === "shutdown")
    {
        interrupt();
    }
});
control.on("close", interrupt);
const root = fileURLToPath(new URL("../../../", import.meta.url));
const suffix = process.platform === "win32" ? ".exe" : "";
const executable = join(root, `target/cargo/debug/nanika-e2e${suffix}`);
const identity = `app.nanika.e2e.${process.pid}.${Date.now()}`;
const output = join(root, "target/verification/e2e", identity);
await mkdir(output, { recursive: true });
cancellation.signal.throwIfAborted();
// Keep the Unix activation socket path below macOS's sockaddr_un limit.
const state = join(root, `target/e2e-${process.pid}-${Date.now().toString(36)}`);
const reservation = Bun.listen({
    hostname: "127.0.0.1",
    port: 0,
    socket: {
        data()
        {}
    }
});
const port = reservation.port;
reservation.stop(true);
console.log(`E2E requires visible native windows and OS input. Output: ${output}`);
const log = Bun.file(join(output, "app.log"));
const child = await OwnedProcess.spawn(
    await processRunner(),
    [executable, "run", state, identity, String(port)],
    root,
    process.env,
    true,
    log
);
let browser: Awaited<ReturnType<typeof remote>> | undefined;
let clipboard: NativeFixture | undefined;
let ime: NativeFixture | undefined;
const assertions: string[] = [];
const failures: unknown[] = [];
try
{
    await _wait(async () =>
    {
        if (!(await Bun.file(join(state, "native-state.json")).exists()))
        {
            return false;
        }
        const native = await _state();
        if (native.pid !== child.pid || native.identity !== identity)
        {
            throw new Error("Native instance identity mismatch");
        }
        try
        {
            return (await fetch(`http://127.0.0.1:${port}/status`, {
                signal: AbortSignal.any([cancellation.signal, AbortSignal.timeout(2000)])
            })).ok;
        }
        catch
        {
            return false;
        }
    }, "driver and native state readiness");
    await _activate();
    browser = await remote({
        hostname: "127.0.0.1",
        port,
        path: "/",
        logLevel: "warn",
        connectionRetryCount: 0,
        connectionRetryTimeout: 10000,
        capabilities: { browserName: "wry", "wdio:enforceWebDriverClassic": true }
    });
    const driver = browser;
    await browser.switchToWindow("launcher");
    const search = await browser.$("input[aria-label='Search for apps and commands']");
    await search.waitForExist({ timeout: 15000 });
    await browser.execute(`
        window.__nanikaE2eInput = [];
        for (const type of ['keydown', 'keyup', 'focus', 'blur', 'compositionstart', 'compositionupdate', 'compositionend']) {
            window.addEventListener(type, event => {
                window.__nanikaE2eInput.push({ type, key: event.key, code: event.code, keyCode: event.keyCode,
                    trusted: event.isTrusted, repeat: event.repeat, composing: event.isComposing,
                    data: event.data, time: performance.now() });
                window.__nanikaE2eInput = window.__nanikaE2eInput.slice(-32);
            }, true);
        }
    `);
    assertions.push("isolated product shell and driver ready");
    await _key("escape", async () => _window("launcher", false));
    for (let index = 0; index < 2; index++)
    {
        await _activate();
        await _key("escape", async () => _window("launcher", false));
    }
    assertions.push(
        `native Escape hides; repeated ${
            scenario === "activation" ? "Rust activation" : "OS shortcut"
        } restores the same instance`
    );
    if (scenario === "ime")
    {
        await _activate();
        await search.setValue("Clipboard History");
        await (await browser.$("[role=option] .title[title='Clipboard History']")).waitForExist({ timeout: 15000 });
        await search.click();
        await browser.execute(`
            window.__nanikaE2eComposition = { active: false, starts: 0, updates: 0, ends: 0, committed: '' };
            for (const type of ['compositionstart', 'compositionupdate', 'compositionend']) {
                window.addEventListener(type, event => {
                    if (!event.isTrusted) return;
                    const state = window.__nanikaE2eComposition;
                    if (type === 'compositionstart') { state.active = true; state.starts++; }
                    else if (type === 'compositionupdate') state.updates++;
                    else { state.active = false; state.ends++; state.committed = event.data; }
                }, true);
            }
        `);
        ime = new NativeFixture([executable, "ime", String(child.pid)], "input source");
        await ime.expect("selected", cancellation.signal);
        await _key("n", async () =>
        {
            await _wait(
                async () => await driver.execute<boolean, []>("return window.__nanikaE2eComposition.active === true"),
                "trusted native IME composition start"
            );
        });
        await _key("i", async () =>
        {
            await _wait(
                async () => await driver.execute<boolean, []>("return window.__nanikaE2eComposition.updates >= 2"),
                "native IME composition update"
            );
        });
        await _key("enter", async () =>
        {
            await _wait(
                async () => await driver.execute<boolean, []>("return window.__nanikaE2eComposition.ends === 1"),
                "native IME composition commit"
            );
        });
        await _window("launcher", true, true);
        const committed = await driver.execute<string, []>("return window.__nanikaE2eComposition.committed");
        if (
            typeof committed !== "string" || !committed || !await search.isExisting()
            || !(await search.getValue()).includes(committed)
        )
        {
            throw new Error("IME Enter submitted a launcher action or discarded the composed text");
        }
        assertions.push("trusted native IME composition commits with Enter without invoking a launcher action");
        await search.setValue("Clipboard History");
        await _key("n", async () =>
        {
            await _wait(
                async () =>
                    await driver.execute<boolean, []>(
                        "return window.__nanikaE2eComposition.active && window.__nanikaE2eComposition.starts === 2"
                    ),
                "second native IME composition"
            );
        });
        await _key("escape", async () =>
        {
            await _wait(
                async () => await driver.execute<boolean, []>("return window.__nanikaE2eComposition.ends === 2"),
                "native IME composition cancellation"
            );
        });
        await _window("launcher", true, true);
        if (!await search.isExisting())
        {
            throw new Error("IME Escape left the launcher search");
        }
        assertions.push("native IME Escape cancels composition without hiding the launcher");
        await _key("escape", async () => _window("launcher", false));
        assertions.push("ordinary Escape after composition still hides the launcher");
        await Bun.write(
            join(output, "composition.json"),
            JSON.stringify(
                await driver.execute("return window.__nanikaE2eInput"),
                null,
                2
            )
        );
    }
    if (scenario === "application")
    {
        await _key("launcher", async () => _window("launcher", true, true));
        await _launcherReady();
        assertions.push("OS global shortcut restores the launcher");
        await search.setValue("System");
        await (await browser.$("[role=option][aria-posinset='2']")).waitForExist({ timeout: 15000 });
        const selected = () => driver.$("[role=option][aria-selected=true]");
        const firstSelection = await (await selected()).getAttribute("id");
        await _key("down", async () =>
        {
            await _wait(
                async () => (await (await selected()).getAttribute("id")) !== firstSelection,
                "next search result"
            );
        });
        await _key("up", async () =>
        {
            await _wait(
                async () => (await (await selected()).getAttribute("id")) === firstSelection,
                "previous search result"
            );
        });
        assertions.push("native arrow keys navigate actual search results");
        await search.setValue("Empty Trash");
        const trash = await browser.$("[role=option] .title[title='Empty Trash']");
        await trash.waitForExist({ timeout: 15000 });
        await trash.click();
        await (await browser.$("[role=alertdialog]")).waitForExist({ timeout: 5000 });
        await _key("escape", async () =>
        {
            await (await driver.$("[role=alertdialog]")).waitForExist({ reverse: true, timeout: 5000 });
        });
        await _window("launcher", true, true);
        assertions.push("system confirmation cancels through native Escape without hiding launcher");
        await trash.click();
        await (await browser.$("[role=alertdialog]")).waitForExist({ timeout: 5000 });
        await _key("switch-app", async () => _window("launcher", false));
        await _key("launcher", async () => _window("launcher", true, true));
        await _launcherReady();
        await (await browser.$("[role=alertdialog]")).waitForExist({ reverse: true, timeout: 5000 });
        if (await search.getValue() !== "Empty Trash")
        {
            throw new Error("Focus loss discarded the launcher's query");
        }
        assertions.push(
            "OS app switching dismisses confirmation; shortcut restores the previous query without a stale dialog"
        );
        await _key("settings", async () => _window("settings", true, true));
        await browser.switchToWindow("settings");
        await (await browser.$("input[aria-label='Search settings']")).waitForExist({ timeout: 10000 });
        await browser.saveScreenshot(join(output, "settings.png"));
        assertions.push("native settings shortcut opens the actual Settings window");
        await _key("launcher", async () => _window("launcher", true, true));
        await _launcherReady();
        await search.setValue("Clipboard History");
        await (await browser.$("[role=option][aria-selected=true] .title[title='Clipboard History']"))
            .waitForExist({ timeout: 15000 });
        await _key("enter", async () =>
        {
            await (await driver.$("button[aria-label='Clipboard History menu']")).waitForExist({ timeout: 10000 });
        });
        assertions.push("native Enter opens the selected extension through the real runtime");
        await (await browser.$("button[aria-label='Clipboard History menu']")).click();
        await (await browser.$("[role=menu]")).waitForExist({ timeout: 5000 });
        const menuItems = await browser.$$("[role=menuitem]");
        const settingsItem = await menuItems.find(async item => (await item.getText()).startsWith("Settings"));
        if (!settingsItem)
        {
            throw new Error("Extension menu has no Settings item");
        }
        await settingsItem.click();
        await _window("settings", true, true);
        await browser.switchToWindow("settings");
        await (await browser.$("input[aria-label='Maximum history entries']")).waitForExist({ timeout: 10000 });
        await browser.saveScreenshot(join(output, "extension-settings.png"));
        assertions.push("extension menu Settings navigates the actual Settings window to Clipboard");
        const sections = await browser.$$("nav[aria-label='Settings sections'] button");
        const applications = await sections.find(async item => (await item.getText()).trim() === "Applications");
        if (!applications)
        {
            throw new Error("Settings has no Applications section");
        }
        await applications.click();
        const directories = await browser.$("[role=group][aria-label=Folders]");
        await directories.waitForExist({ timeout: 10000 });
        const previousPaths = await directories.getText();
        const configFile = Bun.file(join(state, "config/extensions/com.nanika.application/settings.jsonc"));
        const previousConfig = await configFile.exists() ? await configFile.text() : null;
        await (await directories.$("button")).click();
        await _wait(async () => (await _state()).windows.settings?.pickerOpen === true, "native folder picker");
        await _key("escape", async () =>
        {
            await _wait(
                async () => (await _state()).windows.settings?.pickerOpen === false,
                "folder picker cancellation"
            );
        });
        await _window("settings", true, true);
        const currentConfig = await configFile.exists() ? await configFile.text() : null;
        if (await directories.getText() !== previousPaths || currentConfig !== previousConfig)
        {
            throw new Error("Cancelling the folder picker changed configured folders");
        }
        assertions.push("native folder picker opens and cancels without changing configuration");
        await _key("launcher", async () => _window("launcher", true, true));
        await _launcherReady();
        const marker = `Nanika E2E ${identity}`;
        clipboard = new NativeFixture([executable, "clipboard", marker], "clipboard");
        await clipboard.expect("seeded", cancellation.signal);
        const filter = await browser.$("input[aria-label='Search clipboard history']");
        await filter.setValue(marker);
        await browser.waitUntil(async () =>
        {
            const items = await driver.$$("[role=option]");
            const item = items[0];
            return items.length === 1 && item !== undefined && (await item.getText()).includes(marker);
        }, { timeout: 15000 });
        await clipboard.command("replace", "replaced", cancellation.signal);
        await _key("enter", async () => _window("launcher", false));
        await clipboard.command("verify", "verified", cancellation.signal);
        assertions.push("real clipboard capture and selected-entry copy round trip preserve the fixture text");
    }
}
catch (error)
{
    failures.push(error);
    if (browser)
    {
        try
        {
            await browser.saveScreenshot(join(output, "failure.png"));
            await Bun.write(join(output, "failure.html"), await browser.getPageSource());
            await Bun.write(
                join(output, "input.json"),
                JSON.stringify(
                    await browser.execute(`
                return { events: window.__nanikaE2eInput, visible: document.visibilityState,
                    focused: document.hasFocus(), active: document.activeElement?.outerHTML };
            `),
                    null,
                    2
                )
            );
        }
        catch (captureError)
        {
            failures.push(captureError);
        }
    }
}
finally
{
    if (ime)
    {
        try
        {
            await ime.close();
            assertions.push("original OS input source restored");
        }
        catch (error)
        {
            failures.push(error);
        }
    }
    try
    {
        await browser?.deleteSession();
    }
    catch (error)
    {
        failures.push(error);
    }
    let deadline: ReturnType<typeof setTimeout> | undefined;
    try
    {
        try
        {
            await child.control("close");
        }
        catch (error)
        {
            failures.push(error);
        }
        await Promise.race([
            child.exited,
            new Promise<void>(resolve =>
            {
                deadline = setTimeout(resolve, 10000);
            })
        ]);
        await stopProcessTree(child, "SIGTERM");
        if (child.exitCode !== 0)
        {
            failures.push(new Error(`E2E app exited with ${child.signalCode ?? child.exitCode}`));
        }
    }
    catch (error)
    {
        failures.push(error);
    }
    finally
    {
        clearTimeout(deadline);
    }
    if (clipboard)
    {
        try
        {
            await clipboard.close();
            assertions.push("original OS clipboard restored");
        }
        catch (error)
        {
            failures.push(error);
        }
    }
    await Bun.write(
        join(output, "report.json"),
        JSON.stringify(
            {
                scenario,
                assertions,
                failures: failures.map(_describeError),
                platform: process.platform,
                native: await _state().catch(() => null),
                state,
                identity
            },
            null,
            2
        )
    );
    process.removeListener("SIGINT", interrupt);
    process.removeListener("SIGTERM", interrupt);
    control.removeAllListeners();
    control.close();
}
if (failures.length)
{
    throw new AggregateError(failures, `E2E failed: ${output}`);
}
console.log(`E2E passed: ${assertions.length} assertions; ${output}`);

function _describeError(error: unknown): string
{
    if (error instanceof AggregateError)
    {
        return `${error.stack}\n${error.errors.map(_describeError).join("\n")}`;
    }
    return error instanceof Error ? error.stack ?? error.message : String(error);
}

async function _command(...args: string[]): Promise<void>
{
    cancellation.signal.throwIfAborted();
    const process = Bun.spawn([executable, ...args], {
        stdin: "ignore",
        stdout: "ignore",
        stderr: "pipe",
        windowsHide: true,
        signal: cancellation.signal,
        timeout: 15000
    });
    const [exitCode, stderr] = await Promise.all([process.exited, new Response(process.stderr).text()]);
    await appendFile(join(output, "native-input.log"), `${args.join(" ")}: ${exitCode}\n${stderr}`);
    if (exitCode !== 0)
    {
        throw new Error(`Native command failed (${args[0]}): ${stderr}`);
    }
}

async function _state(): Promise<NativeState>
{
    return await Bun.file(join(state, "native-state.json")).json() as NativeState;
}

async function _window(label: string, visible: boolean, focused?: boolean): Promise<void>
{
    await _wait(async () =>
    {
        const window = (await _state()).windows[label];
        return window?.visible === visible && (focused === undefined || window.focused === focused);
    }, `${label} visible=${visible} focused=${focused}`);
}

async function _wait(predicate: () => Promise<boolean>, description: string): Promise<void>
{
    const deadline = Date.now() + 15000;
    while (true)
    {
        cancellation.signal.throwIfAborted();
        if (child.exitCode !== null || child.signalCode !== null)
        {
            throw new Error(`App exited while waiting for ${description}`);
        }
        if (await predicate())
        {
            return;
        }
        if (Date.now() >= deadline)
        {
            throw new Error(`Timed out waiting for ${description}`);
        }
        await Bun.sleep(50);
    }
}

async function _key(key: string, observe: () => Promise<void>): Promise<void>
{
    cancellation.signal.throwIfAborted();
    const sender = Bun.spawn([executable, "key", key], {
        stdin: "pipe",
        stdout: "pipe",
        stderr: "pipe",
        windowsHide: true,
        // Finish posting key-up events even if the scenario is cancelled during a chord.
        timeout: 30000
    });
    const errors = new Response(sender.stderr).text();
    const problems: unknown[] = [];
    const lines = createInterface({ input: Readable.from(sender.stdout) });
    try
    {
        const first = await lines[Symbol.asyncIterator]().next();
        if (first.value !== "posted")
        {
            throw new Error(`Native key ${key} expected posted, received ${first.value ?? "EOF"}`);
        }
        await observe();
    }
    catch (error)
    {
        problems.push(error);
    }
    finally
    {
        lines.close();
        try
        {
            await sender.stdin.end();
        }
        catch (error)
        {
            problems.push(error);
        }
        const exitCode = await sender.exited;
        await appendFile(join(output, "native-input.log"), `key ${key}: ${exitCode}\n${await errors}`);
        if (exitCode !== 0)
        {
            problems.push(new Error(`Native key sender exited with ${exitCode}`));
        }
    }
    if (problems.length)
    {
        throw new AggregateError(problems, `Native key ${key} failed`);
    }
}

async function _activate(): Promise<void>
{
    if (scenario === "activation")
    {
        await _command("activate", state, identity, String(child.pid));
        await _window("launcher", true, true);
    }
    else
    {
        await _key("launcher", async () => _window("launcher", true, true));
    }
    if (browser)
    {
        await _launcherReady();
    }
}

async function _launcherReady(): Promise<void>
{
    if (!browser)
    {
        throw new Error("WebDriver session is not ready");
    }
    const driver = browser;
    await driver.switchToWindow("launcher");
    await driver.waitUntil(
        async () => await driver.execute("return document.visibilityState === 'visible' && document.hasFocus()"),
        { timeout: 5000, timeoutMsg: "Launcher WebView did not regain visibility and focus" }
    );
}
