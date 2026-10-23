import { expect, test } from "vitest";

import { hostResources } from "../../src/generated/host-resources";
import { LocalizationState } from "../../src/localization/LocalizationState.svelte";

test("channel updates received before initialization win over an older initial snapshot", () =>
{
    const state = new LocalizationState(hostResources);
    state.accept(7, { preference: "zh-Hans", locale: "zh-Hans", revision: 3 });
    state.accept(7, { preference: "en", locale: "en", revision: 2 });
    state.initialize(7, { preference: "system", locale: "en", revision: 1 }, "macos");
    expect(state.locale).toBe("zh-Hans");
    expect(state.preference).toBe("zh-Hans");
    expect(state.text({ message: "settings-language" })).toBe("语言");
    state.initialize(7, { preference: "en", locale: "en", revision: 2 }, "macos");
    expect(state.locale).toBe("zh-Hans");
});

test("foreign sessions and stale revisions cannot overwrite current presentation", () =>
{
    const state = new LocalizationState(hostResources);
    state.initialize(2, { preference: "en", locale: "en", revision: 2 }, "windows");
    expect(state.accept(1, { preference: "zh-Hans", locale: "zh-Hans", revision: 100 })).toBe(false);
    expect(state.accept(2, { preference: "zh-Hans", locale: "zh-Hans", revision: 1 })).toBe(false);
    expect(state.text({ message: "settings-language" })).toBe("Language");
    state.initialize(3, { preference: "zh-Hans", locale: "zh-Hans", revision: 1 }, "windows");
    expect(state.locale).toBe("zh-Hans");
    expect(state.accept(2, { preference: "en", locale: "en", revision: 200 })).toBe(false);
});

test("preference-only changes synchronize the control while preserving the effective locale", () =>
{
    const state = new LocalizationState(hostResources);
    state.initialize(1, { preference: "system", locale: "en", revision: 1 }, "macos");
    expect(state.accept(1, { preference: "en", locale: "en", revision: 2 })).toBe(true);
    expect(state.locale).toBe("en");
    expect(state.preference).toBe("en");
    expect(state.text("report.pdf")).toBe("report.pdf");
});

test("preparing a retained menu locale preserves active language and platform", () =>
{
    const state = new LocalizationState(hostResources);
    state.initialize(1, { preference: "system", locale: "en", revision: 1 }, "windows");
    expect(state.text({ message: "settings-title" }, undefined, "zh-Hans")).toBe("设置");
    expect(state.text({ message: "settings-shortcut-modifiers" }, undefined, "zh-Hans")).toContain("Windows");
    expect(state.locale).toBe("en");
    expect(state.preference).toBe("system");
    expect(state.text({ message: "settings-title" })).toBe("Settings");
});
