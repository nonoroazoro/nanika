import type { BrowserCommand } from "./BrowserCommand.ts";

/**
 * Check the shared rail boundary in each mounted product scroll area.
 * @param browser Owned headless session
 */
export async function verifyScrollGutters(browser: BrowserCommand): Promise<void>
{
    await browser(
        "eval",
        `(() => {
        const areas = [...document.querySelectorAll('.scroll-area')].filter(area => area.clientWidth && area.clientHeight);
        if (!areas.length) throw new Error('No product scroll area was exercised');
        return areas.map(area => {
            const viewport = area.querySelector(':scope > .scroll-viewport');
            const track = area.querySelector(':scope > .scroll-track');
            const view = viewport.getBoundingClientRect();
            const rail = track.getBoundingClientRect();
            if (view.right > rail.left - 2 || view.left < area.getBoundingClientRect().left - 1)
                throw new Error('Scroll rail overlaps its content: ' + JSON.stringify({area:area.className, view:view.toJSON(), rail:rail.toJSON()}));
            if (viewport.scrollWidth > viewport.clientWidth + 1) throw new Error('Reserved rail caused horizontal overflow: ' + area.className);
            return {area:area.className, gap:rail.left-view.right, overflow:viewport.scrollHeight>viewport.clientHeight};
        });
    })()`
    );
}
