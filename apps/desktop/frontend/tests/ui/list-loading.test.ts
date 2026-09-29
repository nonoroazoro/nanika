import assert from "node:assert/strict";
import { test } from "vitest";

import { initialListItemCount, listPrefetchDistance } from "../../src/ui/list-loading.ts";

test("new search prefixes cover the viewport and prefetch region at different sizes and scales", () =>
{
    for (const viewport of [180, 434, 620, 1040])
    {
        for (const row of [39, 52, 65, 78.5])
        {
            const count = initialListItemCount(viewport, row);
            assert.ok(count * row > viewport + listPrefetchDistance(viewport));
            assert.ok(count * row <= viewport + listPrefetchDistance(viewport) + (2 * row));
        }
    }
});

test("the published search prefix does not intersect the continuation observer until scrolling", () =>
{
    const viewport = 434;
    const row = 52;
    const count = initialListItemCount(viewport, row);
    const edge = viewport + listPrefetchDistance(viewport);
    assert.equal(count, 27);
    assert.ok(count * row > edge);
    assert.ok((count * row) - (row * 2) <= edge);
});
