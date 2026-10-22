import { get } from "svelte/store";
import { afterEach, expect, test } from "vitest";

import {
    closeConfirmation,
    confirmAction,
    confirmation,
    confirmationPending,
    interruptConfirmation
} from "../../src/ui/confirmation";

afterEach(() =>
{
    interruptConfirmation();
});

test("concurrent requests cannot replace the reviewed operation", async () =>
{
    const first = confirmAction({ title: "Clear history?", label: "Clear", message: "Cannot be undone." });
    expect(confirmationPending()).toBe(true);
    expect(await confirmAction({ title: "Shut down?", label: "Shut down", message: "" })).toBe(false);
    expect(get(confirmation)?.title).toBe("Clear history?");
    closeConfirmation(true)?.();
    interruptConfirmation();
    expect(await first).toBe(true);
    expect(confirmationPending()).toBe(false);
    expect(get(confirmation)).toBeNull();
});

test("cancellation releases the slot and never confirms a later request", async () =>
{
    const first = confirmAction({ title: "Clear?", label: "Clear", message: "" });
    closeConfirmation(false)?.();
    expect(await first).toBe(false);
    const second = confirmAction({ title: "Reset?", label: "Reset", message: "" });
    expect(get(confirmation)?.title).toBe("Reset?");
    closeConfirmation(false)?.();
    expect(await second).toBe(false);
});

test.each([true, false])("decision %s stays pending and immutable until the exit completes", async decision =>
{
    let completed = false;
    const first = confirmAction({ title: "Clear?", label: "Clear", message: "" }).then(value =>
    {
        completed = true;
        return value;
    });
    const finishExit = closeConfirmation(decision);
    expect(finishExit).not.toBeNull();
    expect(get(confirmation)).toBeNull();
    expect(confirmationPending()).toBe(true);
    expect(closeConfirmation(!decision)).toBeNull();
    expect(closeConfirmation(decision)).toBeNull();
    expect(await confirmAction({ title: "Reset?", label: "Reset", message: "" })).toBe(false);
    expect(completed).toBe(false);
    finishExit?.();
    finishExit?.();
    expect(await first).toBe(decision);
    expect(confirmationPending()).toBe(false);
});

test.each([true, false])("interruption preserves decision %s without affecting a replacement request", async decision =>
{
    const first = confirmAction({ title: "Clear?", label: "Clear", message: "" });
    const finishExit = closeConfirmation(decision);
    interruptConfirmation();
    expect(await first).toBe(decision);
    const second = confirmAction({ title: "Reset?", label: "Reset", message: "" });
    finishExit?.();
    expect(confirmationPending()).toBe(true);
    expect(get(confirmation)?.title).toBe("Reset?");
    interruptConfirmation();
    expect(await second).toBe(false);
});

test("interrupting an unanswered modal cancels it and releases the slot", async () =>
{
    const pending = confirmAction({ title: "Clear?", label: "Clear", message: "" });
    interruptConfirmation();
    expect(await pending).toBe(false);
    expect(confirmationPending()).toBe(false);
    expect(get(confirmation)).toBeNull();
    expect(closeConfirmation(true)).toBeNull();
});
