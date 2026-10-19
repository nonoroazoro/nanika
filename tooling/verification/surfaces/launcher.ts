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
