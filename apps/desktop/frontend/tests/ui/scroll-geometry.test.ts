import assert from "node:assert/strict";
import { test } from "vitest";

import { scrollGeometry } from "../../src/ui/scroll-geometry";

test("ordinary collections keep native pixel coordinates", () =>
{
    const geometry = scrollGeometry(520000, 400);
    assert.equal(geometry.extent, 520000);
    assert.equal(geometry.logical(12345), 12345);
    assert.equal(geometry.physical(12345), 12345);
});

test("large collections preserve both endpoints without exceeding layout bounds", () =>
{
    for (const total of [1_000_000 * 52, 0xffffffff * 52])
    {
        const geometry = scrollGeometry(total, 400);
        assert.ok(geometry.extent <= 4_000_000);
        assert.equal(geometry.logical(geometry.extent - 400), total - 400);
        assert.equal(geometry.physical(total - 400), geometry.extent - 400);
        for (const fraction of [0, 0.1, 0.5, 0.9, 1])
        {
            const top = (total - 400) * fraction;
            assert.ok(Math.abs(geometry.logical(geometry.physical(top)) - top) < 0.001);
        }
    }
});

test("empty and short collections have no scroll range", () =>
{
    for (const height of [0, 52, 400])
    {
        const geometry = scrollGeometry(height, 400);
        assert.equal(geometry.logical(100), 0);
        assert.equal(geometry.physical(100), 0);
    }
});
