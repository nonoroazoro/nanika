import assert from "node:assert/strict";
import { test } from "vitest";

import { collectionWindow, itemAt, itemIndex } from "../../src/ui/collection-window";

import type { ListSection } from "../../src/generated/ListSection";

function section(id: string, total: number, offset: number, count: number, title: string | null = null): ListSection
{
    return {
        id,
        title,
        total,
        offset,
        items: Array.from(
            { length: count },
            (_, i) => ({ id: `${id}-${offset + i}`, title: "Item", subtitle: null, icon: null, actions: [] })
        )
    };
}

test("empty delivered windows request rows while complete empty collections do not", () =>
{
    assert.deepEqual(collectionWindow([section("a", 0, 0, 0)], 0, 520, 52, 36), {
        total: 0,
        offset: 0,
        count: 1,
        covered: true
    });
    const pending = collectionWindow([section("a", 10000, 0, 0)], 0, 520, 52, 36);
    assert.equal(pending.covered, false);
    assert.equal(pending.offset, 0);
    assert.equal(pending.count, 31);
});

test("windows prefetch ahead of the viewport and stay bounded after large scroll jumps", () =>
{
    const sections = [section("a", 10000, 0, 31)];
    assert.equal(collectionWindow(sections, 0, 520, 52, 36).covered, true);
    assert.equal(collectionWindow(sections, 1040, 520, 52, 36).covered, false);
    const next = collectionWindow(sections, 52000, 520, 52, 36);
    assert.equal(next.offset, 980);
    assert.equal(next.count, 51);
    assert.equal(collectionWindow(sections, 0, 100000, 52, 36).count, 500);
});

test("section headings and unloaded sections retain absolute item positions", () =>
{
    const sections = [section("a", 20, 18, 2, "A"), section("b", 30, 0, 10, "B")];
    assert.equal(itemAt(sections, 19)?.id, "a-19");
    assert.equal(itemAt(sections, 20)?.id, "b-0");
    assert.equal(itemAt(sections, 0), undefined);
    assert.equal(itemIndex(sections, "b-2"), 22);
    assert.equal(itemIndex(sections, "missing"), null);
    const range = collectionWindow(sections, 36 + 1040 + 36, 104, 52, 36);
    assert.equal(range.offset, 10);
    assert.equal(range.total, 50);
});

test("anchor geometry includes headings when a record moves across sections", async () =>
{
    const { itemTop } = await import("../../src/ui/collection-window");
    const before = [section("a", 2, 0, 2, "A"), section("b", 20, 0, 2, "B")];
    assert.equal(itemTop(before, 0, 52, 28), 28);
    assert.equal(itemTop(before, 2, 52, 28), 160);
    assert.equal(itemTop(before, 3, 52, 28), 212);
});
