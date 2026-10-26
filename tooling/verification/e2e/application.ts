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
// Some Windows installations start Pinyin in Latin mode. The optional shortcut
// is an explicit session preference, not an assumption about every user's IME.
const imeShortcut = process.env.NANIKA_E2E_IME_SHORTCUT;
if (imeShortcut && (process.platform !== "win32" || imeShortcut !== "ctrl-space"))
{
    throw new Error("NANIKA_E2E_IME_SHORTCUT supports ctrl-space on Windows only");
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
let imeShortcutActive = false;
const assertions: string[] = [];
const failures: unknown[] = [];
let restoring = false;
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
        window.__nanikaE2eErrors = [];
        const originalError = console.error;
        console.error = (...values) => {
            window.__nanikaE2eErrors.push(values.map(value => value instanceof Error ? String(value) + '\\n' + value.stack : String(value)).join(' '));
            window.__nanikaE2eErrors = window.__nanikaE2eErrors.slice(-32);
            originalError.apply(console, values);
        };
        for (const type of ['keydown', 'keyup', 'focus', 'blur', 'visibilitychange', 'compositionstart', 'compositionupdate', 'compositionend']) {
            window.addEventListener(type, event => {
                window.__nanikaE2eInput.push({ type, key: event.key, code: event.code, keyCode: event.keyCode,
                    trusted: event.isTrusted, repeat: event.repeat, composing: event.isComposing,
                    meta: event.metaKey, control: event.ctrlKey, alt: event.altKey, shift: event.shiftKey,
                    visible: document.visibilityState, focused: document.hasFocus(),
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
        await _key("settings", async () => _window("settings", true, true));
        await browser.switchToWindow("settings");
        await (await browser.$('[role=combobox][aria-label="Language"]')).waitForExist({ timeout: 10000 });
        await _activate();
        const imeSearch = await browser.$(".launcher input[role=combobox]");
        await imeSearch.setValue("Clipboard History");
        await (await browser.$("[role=option] .title[title='Clipboard History']")).waitForExist({ timeout: 15000 });
        await imeSearch.click();
        await browser.execute(`
            window.__nanikaE2eComposition = { active: false, starts: 0, updates: 0, ends: 0, committed: '' };
            for (const type of ['compositionstart', 'compositionupdate', 'compositionend']) {
                window.addEventListener(type, event => {
                    const state = window.__nanikaE2eComposition;
                    // WebView2 can report an untrusted compositionend after a native
                    // IME commit. Only accept it following a trusted native start.
                    if (!event.isTrusted && (type !== 'compositionend' || !state.active)) return;
                    if (type === 'compositionstart') { state.active = true; state.starts++; }
                    else if (type === 'compositionupdate') state.updates++;
                    else { state.active = false; state.ends++; state.committed = event.data; }
                }, true);
            }
        `);
        ime = new NativeFixture([executable, "ime", String(child.pid)], "input source");
        await ime.expect("selected", cancellation.signal);
        if (imeShortcut)
        {
            await _launcherReady();
            await imeSearch.click();
            await _key(imeShortcut, async () =>
            {
                imeShortcutActive = true;
                await _launcherReady();
            });
        }
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
        await _switchImeLanguage("zh-Hans");
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
            typeof committed !== "string" || !committed || !await imeSearch.isExisting()
            || !(await imeSearch.getValue()).includes(committed)
        )
        {
            throw new Error("IME Enter submitted a launcher action or discarded the composed text");
        }
        assertions.push("trusted native IME composition commits with Enter without invoking a launcher action");
        await imeSearch.setValue("Clipboard History");
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
        await _switchImeLanguage("en");
        await _key("escape", async () =>
        {
            await _wait(
                async () => await driver.execute<boolean, []>("return window.__nanikaE2eComposition.ends === 2"),
                "native IME composition cancellation"
            );
        });
        await _window("launcher", true, true);
        if (!await imeSearch.isExisting())
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
        await _wait(
            async () =>
                await driver.execute(
                    "return document.activeElement === document.querySelector('.launcher input[role=combobox]')"
                ) === true,
            "native restoration of search input focus"
        );
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
        await browser.switchToWindow("launcher");
        await browser.execute(`
            window.__nanikaLocalizationBaseline = {
                input: document.querySelector('input[aria-label="Search for apps and commands"]'),
                query: document.querySelector('input[aria-label="Search for apps and commands"]').value,
                selected: document.querySelector('[role=option][aria-selected=true]')
            };
        `);
        await browser.switchToWindow("settings");
        await _selectLanguage("zh-Hans", "zh-Hans");
        await browser.execute(`
            const names = [...document.querySelectorAll('.sidebar button')].map(button => button.textContent.trim());
            for (const name of ['应用程序', '脚本', '计算器', '剪贴板历史', '系统']) {
                if (!names.includes(name)) throw new Error('Missing packaged extension translation: ' + name);
            }
        `);
        assertions.push("all built-in names render Chinese from packaged resources through authorized metadata reads");
        await browser.saveScreenshot(join(output, "settings-chinese.png"));
        await browser.switchToWindow("launcher");
        await _wait(
            async () => await driver.execute("return document.documentElement.lang === 'zh-Hans'") === true,
            "launcher localization delivery"
        );
        await browser.execute(`
            const before = window.__nanikaLocalizationBaseline;
            if (document.querySelector('input[aria-label="搜索应用和命令"]') !== before.input
                || before.input.value !== before.query
                || document.querySelector('[role=option][aria-selected=true]') !== before.selected) {
                throw new Error('Language switching replaced launcher state');
            }
            if (before.selected.querySelector('.collection-title').textContent !== '清倒废纸篓'
                || before.selected.querySelector('.subtitle').textContent !== '系统'
                || before.selected.querySelector('mark')) {
                throw new Error('Packaged root candidate did not translate coherently');
            }
        `);
        await browser.switchToWindow("settings");
        await _selectLanguage("en", "en");
        assertions.push(
            "real language saves update both retained WebViews, native title and tray labels without replacing launcher query or selection"
        );
        const systemLanguage = await Bun.file(join(state, "system-language.json")).json() as { locale: string; };
        if (systemLanguage.locale !== "en" && systemLanguage.locale !== "zh-Hans")
        {
            throw new Error("Native system-language observation returned an unsupported locale");
        }
        await _selectLanguage("system", systemLanguage.locale);
        await browser.switchToWindow("launcher");
        await _wait(
            async () => await driver.execute("return document.documentElement.lang") === systemLanguage.locale,
            "Follow System delivery to Launcher"
        );
        await browser.execute(`
            const before = window.__nanikaLocalizationBaseline;
            if (document.querySelector('.launcher input[role=combobox]') !== before.input
                || before.input.value !== before.query
                || document.querySelector('[role=option][aria-selected=true]') !== before.selected) {
                throw new Error('Follow System replaced retained launcher state');
            }
        `);
        await Bun.write(join(output, "system-language.json"), Bun.file(join(state, "system-language.json")));
        assertions.push(
            "Follow System saves the preference and converges both retained WebViews and native title on the current OS language"
        );
        await browser.switchToWindow("settings");
        await _selectLanguage("en", "en");
        if (process.platform === "darwin")
        {
            await _verifyApplicationNames();
        }
        const marker = identity;
        const copiedHtml = (await Bun.file(
            join(root, "apps/desktop/frontend/tests/syntax/fixtures/inline-chart.txt")
        ).text()).replace("<title>", `<title>${marker} `);
        clipboard = new NativeFixture([executable, "clipboard", copiedHtml], "clipboard");
        await clipboard.expect("seeded", cancellation.signal);
        await _key("launcher", async () => _window("launcher", true, true));
        await _launcherReady();
        await search.setValue("Clipboard History");
        await (await browser.$("[role=option][aria-selected=true] .title[title='Clipboard History']"))
            .waitForExist({ timeout: 15000 });
        await _key("enter", async () =>
        {
            await (await driver.$("button[aria-label='\u2068Clipboard History\u2069 menu']")).waitForExist({
                timeout: 10000
            });
        });
        assertions.push("native Enter opens the selected extension through the real runtime");
        const filter = await browser.$("input[aria-label='Search clipboard history']");
        await filter.setValue(marker);
        await browser.waitUntil(async () =>
        {
            const items = await driver.$$("[role=option]");
            const item = items[0];
            return items.length === 1 && item !== undefined && (await item.getText()).includes(marker);
        }, { timeout: 15000 });
        await browser.waitUntil(async () =>
        {
            return (await driver.$$(".text-document [class^='syntax-token-']")).length > 0;
        }, { timeout: 5000 });
        const preview = await browser.$(".text-document");
        if (await preview.getProperty("textContent") !== copiedHtml)
        {
            throw new Error("First Clipboard HTML preview changed the copied source");
        }
        assertions.push("first Clipboard HTML preview highlights from a cold worker without copying again");
        const openedAt = await browser.execute<number, []>("return performance.timeOrigin");
        // The first preview previously triggered a delayed document reload in dev.
        // Keep the real runtime open beyond that delay to cover packaged behavior too.
        await Bun.sleep(3000);
        cancellation.signal.throwIfAborted();
        if (
            await browser.execute<number, []>("return performance.timeOrigin") !== openedAt
            || !await filter.isExisting() || await filter.getValue() !== marker
        )
        {
            throw new Error("First Clipboard preview reloaded or returned to the launcher");
        }
        assertions.push("first Clipboard preview remains in the same document and extension after three seconds");
        await (await browser.$("button[aria-label='\u2068Clipboard History\u2069 menu']")).click();
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
        await (await directories.$(".directory-toolbar button")).click();
        await _wait(_pickerOpen, "native folder picker");
        await _key("escape", async () =>
        {
            await _wait(
                async () => !await _pickerOpen(),
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
                return { events: window.__nanikaE2eInput, errors: window.__nanikaE2eErrors, visible: document.visibilityState,
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
    // Cancellation stops the scenario, but must not cancel OS-state restoration.
    restoring = true;
    if (process.platform === "win32" && child.exitCode === null)
    {
        try
        {
            // Close only this test process's native dialogs before requesting
            // shutdown, which otherwise waits behind the modal message loop.
            await _command("cancel-picker", String(child.pid));
        }
        catch (error)
        {
            failures.push(error);
        }
    }
    if (ime)
    {
        try
        {
            try
            {
                if (imeShortcutActive && imeShortcut)
                {
                    if (!(await _state()).windows.launcher?.visible)
                    {
                        await _activate();
                    }
                    await _launcherReady();
                    await _key(imeShortcut, async () => _window("launcher", true, true));
                    imeShortcutActive = false;
                }
            }
            finally
            {
                await ime.close();
            }
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

async function _command(...args: string[]): Promise<string>
{
    if (!restoring)
    {
        cancellation.signal.throwIfAborted();
    }
    const process = Bun.spawn([executable, ...args], {
        stdin: "ignore",
        stdout: "pipe",
        stderr: "pipe",
        windowsHide: true,
        signal: restoring ? undefined : cancellation.signal,
        timeout: 15000
    });
    const [exitCode, stderr, stdout] = await Promise.all([
        process.exited,
        new Response(process.stderr).text(),
        new Response(process.stdout).text()
    ]);
    await appendFile(join(output, "native-input.log"), `${args.join(" ")}: ${exitCode}\n${stderr}`);
    if (exitCode !== 0)
    {
        throw new Error(`Native command failed (${args[0]}): ${stderr}`);
    }
    return stdout.trim();
}

async function _pickerOpen(): Promise<boolean>
{
    return process.platform === "win32"
        ? await _command("picker", String(child.pid)) === "true"
        : (await _state()).windows.settings?.pickerOpen === true;
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
        if (!restoring)
        {
            cancellation.signal.throwIfAborted();
        }
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
    if (!restoring)
    {
        cancellation.signal.throwIfAborted();
    }
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

async function _switchImeLanguage(locale: "en" | "zh-Hans"): Promise<void>
{
    if (!browser)
    {
        throw new Error("WebDriver session is not ready");
    }
    const driver = browser;
    await driver.execute(`
        const input = document.querySelector('.launcher input[role=combobox]');
        const composition = window.__nanikaE2eComposition;
        if (!composition.active || document.activeElement !== input || !document.hasFocus()) {
            throw new Error('Expected focused native composition before language save');
        }
        window.__nanikaE2eImeBaseline = {
            input, value: input.value, start: input.selectionStart, end: input.selectionEnd,
            direction: input.selectionDirection, starts: composition.starts, ends: composition.ends,
            updates: composition.updates
        };
    `);
    // The maintained driver's window switch selects a WebView without changing OS focus.
    // Invoke from Settings because the real command authorizes that window as its caller.
    await driver.switchToWindow("settings");
    await driver.execute(
        `
        const locale = arguments[0];
        window.__nanikaE2eLanguageSave = null;
        const done = result => { window.__nanikaE2eLanguageSave = result; };
        window.__TAURI_INTERNALS__.invoke('save_host_settings', {
            request: { key: 'language', value: locale }
        }).then(result => {
            done(!result.error && result.saved.language === locale && result.effective?.language === locale
                ? 'saved' : JSON.stringify(result));
        }, error => done(JSON.stringify(error)));
    `,
        locale
    );
    await _wait(
        async () => await driver.execute("return window.__nanikaE2eLanguageSave !== null"),
        "language save completion during native composition"
    );
    const result = await driver.execute<string, []>("return window.__nanikaE2eLanguageSave");
    if (result !== "saved")
    {
        throw new Error(`Language save during native composition failed: ${result}`);
    }
    await driver.switchToWindow("launcher");
    await _wait(
        async () => await driver.execute("return document.documentElement.lang") === locale,
        "language delivery during native composition"
    );
    await _window("launcher", true, true);
    await driver.execute(`
        const before = window.__nanikaE2eImeBaseline;
        const input = document.querySelector('.launcher input[role=combobox]');
        const composition = window.__nanikaE2eComposition;
        if (input !== before.input || document.activeElement !== input || !document.hasFocus()
            || input.value !== before.value || input.selectionStart !== before.start
            || input.selectionEnd !== before.end || input.selectionDirection !== before.direction
            || !composition.active || composition.starts !== before.starts
            || composition.ends !== before.ends || composition.updates !== before.updates) {
            throw new Error('Language save interrupted native composition or replaced its input state');
        }
    `);
    assertions.push(
        `real language save to ${locale} preserves native composition, input identity, focus and selection`
    );
}

async function _selectLanguage(preference: "en" | "system" | "zh-Hans", locale: "en" | "zh-Hans"): Promise<void>
{
    if (!browser)
    {
        throw new Error("WebDriver session is not ready");
    }
    const driver = browser;
    // Driver click focuses the control; native keys exercise the actual Select contract.
    await (await driver.$('[role=combobox][aria-label="Language"], [role=combobox][aria-label="语言"]')).click();
    const highlighted = () => driver.$("[role=option][data-highlighted]");
    await _key("enter", async () =>
    {
        await (await highlighted()).waitForExist({ timeout: 5000 });
    });
    const options = ["system", "en", "zh-Hans"];
    let current = options.indexOf(await (await highlighted()).getAttribute("data-value") ?? "");
    const target = options.indexOf(preference);
    if (current < 0)
    {
        throw new Error("Language selector has no recognized highlighted option");
    }
    while (current !== target)
    {
        const direction = current < target ? "down" : "up";
        current += current < target ? 1 : -1;
        const expected = options[current];
        await _key(direction, async () =>
        {
            await _wait(
                async () => await (await highlighted()).getAttribute("data-value") === expected,
                "language option"
            );
        });
    }
    await _key("enter", async () =>
    {
        await _wait(
            async () =>
                (await Bun.file(join(state, "config/nanika.jsonc")).text())
                    .match(/"language"\s*:\s*"([^"]+)"/u)?.[1] === preference,
            "persisted language preference"
        );
    });
    await _wait(
        async () => await driver.execute("return document.documentElement.lang") === locale,
        "Settings language delivery"
    );
    await _wait(
        async () =>
        {
            const native = await _state();
            return native.windows.settings?.title === (locale === "en" ? "Nanika Settings" : "Nanika 设置")
                && native.trayLabels[0] === (locale === "en" ? "Open Nanika" : "打开 Nanika")
                && native.trayLabels[1] === (locale === "en" ? "Quit" : "退出");
        },
        "localized native Settings title and tray labels"
    );
}

async function _verifyApplicationNames(): Promise<void>
{
    const driver = browser;
    if (!driver)
    {
        throw new Error("Application driver is unavailable");
    }
    await _key("launcher", async () => _window("launcher", true, true));
    await _launcherReady();
    const search = await driver.$(".launcher input[role=combobox]");
    await search.setValue("Nanika NetEaseMusic");
    await (await driver.$("[role=option] .title[title='Nanika NetEaseMusic']"))
        .waitForExist({ timeout: 15000 });
    await driver.execute(`
        window.__nanikaApplicationNameBaseline = {
            input: document.querySelector('.launcher input[role=combobox]'),
            selected: document.querySelector('[role=option][aria-selected=true]')
        };
    `);
    await _key("settings", async () => _window("settings", true, true));
    await driver.switchToWindow("settings");
    await _selectLanguage("zh-Hans", "zh-Hans");
    await driver.switchToWindow("launcher");
    await (await driver.$("[role=option] .title[title='Nanika 网易云音乐']"))
        .waitForExist({ timeout: 15000 });
    await driver.execute(`
        const before = window.__nanikaApplicationNameBaseline;
        if (document.querySelector('.launcher input[role=combobox]') !== before.input
            || before.input.value !== 'Nanika NetEaseMusic'
            || document.querySelector('[role=option][aria-selected=true]') !== before.selected) {
            throw new Error('Native application translation replaced query, input or selection');
        }
    `);
    await _key("launcher", async () => _window("launcher", true, true));
    await _launcherReady();
    for (const query of ["网易", "音乐", "wy", "yy", "netease", "music", "ミュージック"])
    {
        await search.setValue(query);
        await (await driver.$("[role=option] .title[title='Nanika 网易云音乐']"))
            .waitForExist({ timeout: 15000 });
    }
    for (
        const [query, title] of [
            ["網易", "Nanika 網易雲音樂"],
            ["traditional", "Nanika 網易雲音樂"],
            ["undetermined", "Nanika 網易雲音樂"],
            ["american", "Nanika 網易雲音樂"],
            ["音楽箱", "Nanika 音楽箱"]
        ] as const
    )
    {
        await search.setValue(query);
        await (await driver.$(`[role=option] .title[title='${title}']`))
            .waitForExist({ timeout: 15000 });
    }
    await search.setValue("Nanika");
    await _key("settings", async () => _window("settings", true, true));
    await driver.switchToWindow("settings");
    await _selectLanguage("en", "en");
    await driver.switchToWindow("launcher");
    await (await driver.$("[role=option] .title[title='Nanika Traditional Music']"))
        .waitForExist({ timeout: 15000 });
    await (await driver.$("[role=option] .title[title='Nanika 音楽箱']"))
        .waitForExist({ timeout: 15000 });
    await (await driver.$("[role=option] .title[title='Nanika NetEaseMusic']"))
        .waitForExist({ timeout: 15000 });
    assertions.push(
        "English cold-start native App names switch to Chinese without replacing input or selection; all source names remain searchable; Traditional Chinese and Japanese-only entries remain visible; empty defaults survive discovery and unknown languages never outrank English"
    );
    await driver.switchToWindow("settings");
}
