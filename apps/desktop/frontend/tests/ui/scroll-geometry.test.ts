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

test("ordinary collection rows retain native top and bottom bounce without coordinate compensation", () =>
{
    const geometry = scrollGeometry(5200, 400);
    for (const top of [-80, -24.5, -1, 0, 120, 4800, 4801, 4824.5, 4880])
    {
        assert.equal(geometry.coordinateShift(top), 0);
        // Native scrolling already displaces the track by -scrollTop. Row offsets
        // must stay fixed as the elastic position approaches and crosses an edge.
        const row = top < 0 ? 0 : 5148;
        assert.equal(row - geometry.coordinateShift(top) - top, row - top);
    }
});

test("compressed tracks preserve elastic displacement and exact keyboard anchors independently", () =>
{
    const total = 1_000_000 * 52;
    const geometry = scrollGeometry(total, 400);
    const end = geometry.extent - 400;
    for (const distance of [0, 1, 24.5, 80])
    {
        assert.equal(geometry.coordinateShift(-distance), 0);
        assert.equal(geometry.coordinateShift(end + distance), total - geometry.extent);
        assert.equal(total - geometry.coordinateShift(end + distance) - (end + distance), 400 - distance);
    }
    const anchor = 12345678.125;
    const quantized = Math.round(geometry.physical(anchor));
    assert.equal(geometry.coordinateShift(quantized, anchor), anchor - quantized);
});
