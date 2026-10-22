import { join } from "node:path";

import type { BrowserCommand } from "../BrowserCommand.ts";

/**
 * Exercises the real launcher surface with deterministic bridge fixtures.
 *
 * @param browser Owned headless session
 * @param output Screenshot directory
 */
export async function verifyLauncher(browser: BrowserCommand, output: string): Promise<void>
{
    const search = "input[aria-label='Search for apps and commands']";
    await browser("fill", search, "Empty");
    await browser("wait", "[role=option][aria-selected=true]");
    await browser(
        "eval",
        `(async () => {
        const search = document.querySelector("input[aria-label='Search for apps and commands']");
        search.dispatchEvent(new KeyboardEvent('keydown', {
            key: 'Enter', code: 'Enter', keyCode: 229, isComposing: false, bubbles: true, cancelable: true
        }));
        await new Promise(requestAnimationFrame);
        if (document.querySelector('[role=alertdialog]')) throw new Error('IME Enter invoked a search result');
        return true;
    })()`
    );
    await browser("press", "Enter");
    await browser("wait", "[role=alertdialog]");
    await browser("screenshot", join(output, "system-confirmation.png"));
    await browser("press", "Escape");
    await browser("wait", "--fn", "!document.querySelector('[role=alertdialog]')");
    for (const reduced of [false, true])
    {
        await browser("set", "media", "dark", ...(reduced ? ["reduced-motion"] : []));
        await _verifyConfirmationExit(browser, true, reduced);
        await _verifyConfirmationExit(browser, false, reduced);
    }
    await browser("set", "media", "dark");
    await _verifyConfirmationInterruption(browser);
    await browser("fill", search, "Clipboard");
    await browser("press", "Enter");
    await browser("wait", "--text", "Code preview");
    await browser("snapshot", "-i");
    await browser("screenshot", join(output, "extension.png"));
    await browser("find", "role", "button", "click", "--name", "Clipboard History menu", "--exact");
    await browser("wait", "[role=menu]");
    await browser("screenshot", join(output, "extension-menu.png"));
    await browser("find", "role", "menuitemcheckbox", "click", "--name", "Syntax highlighting", "--exact");
    await browser("wait", "--fn", "!document.querySelector('[role=menu]')");
    await browser("find", "role", "button", "click", "--name", "Clipboard History menu", "--exact");
    await browser("wait", "[role=menuitemcheckbox][aria-checked=false]");
    await browser("find", "role", "menuitem", "click", "--name", "Settings", "--exact");
    await browser("wait", "input[aria-label='Maximum history entries']");
    await browser("screenshot", join(output, "extension-settings.png"));
}

async function _verifyConfirmationExit(browser: BrowserCommand, confirmed: boolean, reduced: boolean): Promise<void>
{
    const search = "input[aria-label='Search for apps and commands']";
    await browser("fill", search, "Empty");
    await browser("press", "Enter");
    await browser("wait", "[role=alertdialog]");
    await browser(
        "wait",
        "--fn",
        "Number(getComputedStyle(document.querySelector('[role=alertdialog]')).opacity) === 1"
    );
    await browser(
        "eval",
        `(() => {
        const root = document.documentElement;
        const search = document.querySelector("input[aria-label='Search for apps and commands']");
        const initialCount = Number(root.dataset.invocationCount ?? 0);
        delete root.dataset.dismissed;
        delete root.dataset.confirmationAtInvoke;
        window.confirmationExit = new Promise((resolve, reject) => {
            const frames = [];
            let exercised = false;
            const deadline = performance.now() + 3000;
            const sample = () => {
                try {
                    const dialog = document.querySelector('[role=alertdialog]');
                    const opacity = dialog ? Number(getComputedStyle(dialog).opacity) : 0;
                    frames.push(opacity);
                    if (dialog?.hasAttribute('data-ending-style')) {
                        if (!document.body.inert || document.activeElement === search) {
                            throw new Error('Confirmation released input before its exit completed');
                        }
                        if (${reduced} && getComputedStyle(dialog).transform !== 'none') {
                            throw new Error('Reduced motion retained the dialog scale');
                        }
                        if (!exercised && opacity > 0 && opacity < 1) {
                            exercised = true;
                            search.focus();
                            if (document.activeElement === search) throw new Error('Closing modal allowed search focus');
                            for (const key of ['x', 'Escape', 'Tab', 'Enter']) {
                                const event = new KeyboardEvent('keydown', { key, repeat: true, bubbles: true, cancelable: true });
                                if (document.activeElement.dispatchEvent(event)) throw new Error('Closing modal leaked ' + key);
                            }
                            // Synthetic clicks bypass inert; the recorded choice must still be immutable.
                            for (const button of dialog.querySelectorAll('button')) button.click();
                        }
                    }
                    if (opacity > 0 && (root.dataset.dismissed || root.dataset.confirmationAtInvoke)) {
                        throw new Error('Confirmation exit was cut short');
                    }
                    if (!dialog) {
                        const count = Number(root.dataset.invocationCount ?? 0) - initialCount;
                        const receipt = root.dataset.confirmationAtInvoke ? JSON.parse(root.dataset.confirmationAtInvoke) : null;
                        if (!exercised || document.body.inert || document.activeElement !== search || search.value !== 'Empty') {
                            throw new Error('Confirmation did not restore unchanged search after the complete exit');
                        }
                        if (count !== ${confirmed ? 1 : 0} || Boolean(root.dataset.dismissed) !== ${confirmed}
                            || (${confirmed} && (!receipt?.confirmed || receipt.opacity !== 0))) {
                            throw new Error('Confirmation lost or changed its recorded decision');
                        }
                        resolve({ confirmed: ${confirmed}, reduced: ${reduced}, frames, receipt, count });
                    } else if (performance.now() > deadline) throw new Error('Confirmation did not finish');
                    else requestAnimationFrame(sample);
                } catch (error) { reject(error); }
            };
            requestAnimationFrame(sample);
        });
        return true;
    })()`
    );
    await browser("press", confirmed ? "Enter" : "Escape");
    await browser("eval", "window.confirmationExit");
}

async function _verifyConfirmationInterruption(browser: BrowserCommand): Promise<void>
{
    await browser("fill", "input[aria-label='Search for apps and commands']", "Empty");
    await browser("press", "Enter");
    await browser("wait", "[role=alertdialog]");
    await browser(
        "wait",
        "--fn",
        "Number(getComputedStyle(document.querySelector('[role=alertdialog]')).opacity) === 1"
    );
    await browser(
        "eval",
        `(async () => {
        const root = document.documentElement;
        const initialCount = Number(root.dataset.invocationCount ?? 0);
        const dialog = document.querySelector('[role=alertdialog]');
        [...dialog.querySelectorAll('button')].find(button => button.textContent.trim() === 'Empty Trash').click();
        if (!document.body.inert) throw new Error('Confirmation did not acquire input isolation');
        await new Promise(requestAnimationFrame);
        window.dispatchEvent(new Event('blur'));
        if (document.body.inert) throw new Error('Interrupted confirmation retained input isolation');
        if (document.activeElement.matches('input')) throw new Error('Interrupted confirmation restored focus');
        await new Promise(requestAnimationFrame);
        if (Number(root.dataset.invocationCount) !== initialCount + 1) {
            throw new Error('Interrupted confirmation lost its accepted invocation');
        }
        return true;
    })()`
    );
    await browser("wait", "--fn", "!document.querySelector('[role=alertdialog]')");
    await browser(
        "eval",
        "(() => { if (document.body.inert || document.activeElement.matches('input')) throw new Error('Stale exit restored focus after interruption'); return true; })()"
    );
}
