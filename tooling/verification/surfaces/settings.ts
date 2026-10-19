import { join } from "node:path";

import type { BrowserCommand } from "../BrowserCommand.ts";

/**
 * Exercises the real settings surface with deterministic bridge fixtures.
 *
 * @param browser Owned headless session
 * @param output Screenshot directory
 */
export async function verifySettings(browser: BrowserCommand, output: string): Promise<void>
{
    await browser("fill", "input[aria-label='Maximum history entries']", "75");
    await browser("find", "role", "button", "click", "--name", "Maximum history entries: Unlimited", "--exact");
    await browser(
        "wait",
        "--fn",
        "document.querySelector('input[aria-label=\"Maximum history entries\"]').readOnly"
    );
    await browser(
        "eval",
        `(() => {
        const input = document.querySelector('input[aria-label="Maximum history entries"]');
        if (input.value !== '75') throw new Error('Unlimited discarded the configured value');
        return true;
    })()`
    );
    await browser(
        "find",
        "role",
        "button",
        "click",
        "--name",
        "Maximum history entries: Use limit 75",
        "--exact"
    );
    await browser(
        "wait",
        "--fn",
        "!document.querySelector('input[aria-label=\"Maximum history entries\"]').readOnly"
    );
    await browser("find", "role", "button", "click", "--name", "Add folder", "--exact");
    await browser("wait", "--text", "/Example/Added folder");
    await browser("find", "role", "button", "click", "--name", "Clear", "--exact");
    await browser("wait", "[role=alertdialog]");
    await browser(
        "eval",
        `(() => {
        const confirm = document.activeElement;
        if (!confirm?.textContent?.includes('Clear history')) throw new Error('Confirm must own initial focus');
        const frame = confirm.closest('.button-focus-frame');
        if (!frame || getComputedStyle(frame).outlineWidth !== '1px') throw new Error('Confirm needs its thin focus frame');
        return { focused: confirm.textContent, outline: getComputedStyle(frame).outlineWidth };
    })()`
    );
    await browser("screenshot", join(output, "confirmation.png"));
    await browser("hover", ".confirmation-actions .button-focus-frame button");
    await browser("mouse", "down");
    await browser(
        "wait",
        "--fn",
        "new DOMMatrix(getComputedStyle(document.querySelector('.button-focus-frame button')).transform).a < 0.99"
    );
    await browser(
        "eval",
        `(() => {
        const frame = document.querySelector('.button-focus-frame');
        if (getComputedStyle(frame).transform !== 'none') throw new Error('The focus frame must not shrink');
        if (getComputedStyle(frame).outlineWidth !== '1px') throw new Error('Pressing must preserve the focus frame');
        return true;
    })()`
    );
    await browser("screenshot", join(output, "confirmation-pressed.png"));
    await browser("mouse", "move", "8", "8");
    await browser("mouse", "up");
    await browser(
        "eval",
        "(() => { if (document.documentElement.dataset.confirmed) throw new Error('Cancelled press confirmed the action'); return true; })()"
    );
    await browser("press", "Escape");
    await browser("wait", "--fn", "!document.querySelector('[role=alertdialog]')");
    await browser(
        "eval",
        "(() => { if (document.activeElement?.textContent?.trim() !== 'Clear') throw new Error('Dialog did not restore trigger focus'); return true; })()"
    );
    await browser("find", "role", "button", "click", "--name", "Clear", "--exact");
    await browser("wait", "[role=alertdialog]");
    await browser("mouse", "move", "8", "8");
    await browser("mouse", "down");
    await browser("mouse", "up");
    await browser("wait", "--fn", "!document.querySelector('[role=alertdialog]')");
    await browser("find", "role", "button", "click", "--name", "Clear", "--exact");
    await browser("wait", "[role=alertdialog]");
    await browser("press", "Enter");
    await browser("wait", "--fn", "document.documentElement.dataset.confirmed === 'true'");
    await browser("fill", "input[placeholder='Search']", "clip");
    await browser("snapshot", "-i");
    await browser("screenshot", join(output, "search-results.png"));
    await browser("set", "media", "dark", "reduced-motion");
    await browser("find", "role", "button", "click", "--name", "Clear", "--exact");
    await browser("wait", "[role=alertdialog]");
    await browser("hover", ".confirmation-actions .button-focus-frame button");
    await browser("mouse", "down");
    await browser(
        "eval",
        `(() => {
        if (!matchMedia('(prefers-reduced-motion: reduce)').matches) throw new Error('Reduced motion is not active');
        for (const element of document.querySelectorAll('.confirmation-dialog, .button-focus-frame button')) {
            if (getComputedStyle(element).transform !== 'none') throw new Error('Reduced motion still changes geometry');
        }
        return true;
    })()`
    );
    await browser("mouse", "move", "8", "8");
    await browser("mouse", "up");
    await browser("press", "Escape");
    await browser("wait", "--fn", "!document.querySelector('[role=alertdialog]')");
    await browser("set", "media", "dark");
    await browser("set", "viewport", "700", "500");
    await browser(
        "eval",
        `(() => {
        if (document.documentElement.scrollWidth > innerWidth) throw new Error('Settings overflows its minimum window width');
        const content = document.querySelector('[aria-label="Settings content"]');
        if (!content || content.getBoundingClientRect().right > innerWidth) throw new Error('Settings content extends outside the window');
        return true;
    })()`
    );
    await browser("screenshot", join(output, "settings-minimum-size.png"));
    await browser("set", "viewport", "920", "680");
}
