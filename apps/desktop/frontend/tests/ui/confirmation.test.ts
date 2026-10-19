import { get } from "svelte/store";
import { afterEach, expect, test } from "vitest";

import { confirmAction, confirmation, confirmationPending, settleConfirmation } from "../../src/ui/confirmation";

afterEach(() =>
{
    settleConfirmation(false);
});

test("concurrent requests cannot replace the reviewed operation", async () =>
{
    const first = confirmAction({ title: "Clear history?", label: "Clear", message: "Cannot be undone." });
    expect(confirmationPending()).toBe(true);
    expect(await confirmAction({ title: "Shut down?", label: "Shut down", message: "" })).toBe(false);
    expect(get(confirmation)?.title).toBe("Clear history?");
    settleConfirmation(true);
    settleConfirmation(false);
    expect(await first).toBe(true);
    expect(confirmationPending()).toBe(false);
    expect(get(confirmation)).toBeNull();
});

test("cancellation releases the slot and never confirms a later request", async () =>
{
    const first = confirmAction({ title: "Clear?", label: "Clear", message: "" });
    settleConfirmation(false);
    expect(await first).toBe(false);
    const second = confirmAction({ title: "Reset?", label: "Reset", message: "" });
    expect(get(confirmation)?.title).toBe("Reset?");
    settleConfirmation(false);
    expect(await second).toBe(false);
});
