import { join } from "node:path";

import { verifyScrollGutters } from "../scroll-area.ts";

import type { BrowserCommand } from "../BrowserCommand.ts";

/**
 * Exercises the real syntax surface with deterministic bridge fixtures.
 *
 * @param browser Owned headless session
 * @param output Screenshot directory
 */
export async function verifySyntax(browser: BrowserCommand, output: string): Promise<void>
{
    await browser("wait", "--fn", "window.syntaxReady === true");
    await browser("eval", "window.verifySyntaxPresentation()");
    await browser(
        "eval",
        "window.prepareSyntaxInteraction().then(value => { window.selectionStart = value.start; return { start: value.start }; })"
    );
    await browser("press", "Shift+ArrowRight");
    await browser("press", "Shift+ArrowRight");
    await browser(
        "eval",
        "(() => { const state = window.syntaxInteractionState(); if (state.focus !== window.selectionStart + 2 || state.anchor !== window.selectionStart) throw new Error('Keyboard selection failed'); return true; })()"
    );
    await verifyScrollGutters(browser);
    await browser("screenshot", join(output, "syntax-selection.png"));
}
