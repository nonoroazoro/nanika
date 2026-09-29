import assert from "node:assert/strict";
import { test } from "vitest";

import { revealSetting } from "../../src/settings/reveal.ts";

test("field reveal scrolls only the owning viewport, accounting for chrome and existing scroll", () =>
{
    const viewport = {
        scrollTop: 120,
        clientTop: 1,
        clientHeight: 400,
        scrollHeight: 1200,
        getBoundingClientRect: () => _rect(48, 400)
    };
    const field = { getBoundingClientRect: () => _rect(649, 80) };
    revealSetting(viewport, field);
    assert.equal(viewport.scrollTop, 560);
});

test("short pages remain at the top instead of displacing titlebar ancestors", () =>
{
    const viewport = {
        scrollTop: 0,
        clientTop: 0,
        clientHeight: 650,
        scrollHeight: 650,
        getBoundingClientRect: () => _rect(48, 650)
    };
    for (const top of [48, 64, 400])
    {
        revealSetting(viewport, { getBoundingClientRect: () => _rect(top, 80) });
        assert.equal(viewport.scrollTop, 0);
    }
});

test("reveal clamps at both ends and aligns oversized fields at their start", () =>
{
    const viewport = {
        scrollTop: 100,
        clientTop: 0,
        clientHeight: 400,
        scrollHeight: 1000,
        getBoundingClientRect: () => _rect(48, 400)
    };
    revealSetting(viewport, { getBoundingClientRect: () => _rect(-52, 40) });
    assert.equal(viewport.scrollTop, 0);
    revealSetting(viewport, { getBoundingClientRect: () => _rect(1028, 20) });
    assert.equal(viewport.scrollTop, 600);
    revealSetting(viewport, { getBoundingClientRect: () => _rect(-352, 500) });
    assert.equal(viewport.scrollTop, 200);
});

function _rect(top: number, height: number): DOMRect
{
    return {
        x: 0,
        y: top,
        top,
        bottom: top + height,
        left: 0,
        right: 500,
        width: 500,
        height,
        toJSON: () => ({ top, height })
    };
}
