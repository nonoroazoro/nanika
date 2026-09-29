import assert from "node:assert/strict";
import { test } from "vitest";

import { settingsAnchor } from "../../src/settings/anchor.ts";

test("stable destinations separate page, section, fields and host extension controls", () =>
{
    const anchors = [
        settingsAnchor("test.extension", { kind: "page" }),
        settingsAnchor("test.extension", { kind: "enabled" }),
        settingsAnchor("test.extension", { kind: "field", key: "enabled" }),
        settingsAnchor("test.extension", { kind: "section", key: "enabled" }),
        settingsAnchor("other.extension", { kind: "field", key: "enabled" })
    ];
    assert.equal(new Set(anchors).size, anchors.length);
    assert.notEqual(
        settingsAnchor("a/b", { kind: "field", key: "c" }),
        settingsAnchor("a", { kind: "field", key: "b/c" })
    );
});
