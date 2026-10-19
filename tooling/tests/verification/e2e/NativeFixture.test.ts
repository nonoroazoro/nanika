import { mkdir, mkdtemp, rm } from "node:fs/promises";
import { join, resolve } from "node:path";
import { expect, test } from "vitest";

import { OwnedProcess } from "../../../quality/OwnedProcess.ts";
import { processRunner } from "../../../quality/process-tree.ts";
import { NativeFixture } from "../../../verification/e2e/NativeFixture.ts";

test("a crashed scenario leaves its contained fixture alive until EOF restoration completes", async () =>
{
    const root = resolve("target/test-work");
    await mkdir(root, { recursive: true });
    const directory = await mkdtemp(join(root, "fixture-crash-"));
    const fixtureModule = resolve(import.meta.dirname, "../../../verification/e2e/NativeFixture.ts");
    await Bun.write(
        join(directory, "fixture.ts"),
        `
        console.log("seeded");
        await Bun.stdin.text();
        await Bun.sleep(250);
        await Bun.write("restored", "original state");
    `
    );
    await Bun.write(
        join(directory, "scenario.ts"),
        `
        import { NativeFixture } from ${JSON.stringify(fixtureModule)};
        const fixture = new NativeFixture(["bun", "fixture.ts"], "test state");
        await fixture.expect("seeded", new AbortController().signal);
        process.exit(23);
    `
    );
    const scenario = await OwnedProcess.spawn(
        await processRunner(),
        ["bun", "scenario.ts"],
        directory,
        process.env,
        true
    );
    try
    {
        await scenario.exited;
        expect(scenario.exitCode).toBe(23);
        expect(await Bun.file(join(directory, "restored")).text()).toBe("original state");
    }
    finally
    {
        await scenario.control("stop");
        await scenario.exited;
        await rm(directory, { recursive: true });
    }
}, 15000);

test("closing the fixture waits for restoration after a successful exchange", async () =>
{
    const root = resolve("target/test-work");
    await mkdir(root, { recursive: true });
    const directory = await mkdtemp(join(root, "clipboard-owner-"));
    const restored = join(directory, "restored");
    const fixture = new NativeFixture([
        "bun",
        "-e",
        `
        const { createInterface } = await import("node:readline");
        console.log("seeded");
        for await (const line of createInterface({ input: process.stdin })) {
            if (line === "replace") console.log("replaced");
            else if (line === "verify") console.log("verified");
            else process.exit(1);
        }
        await Bun.write(process.argv[1], "restored");
        `,
        restored
    ], "clipboard");
    try
    {
        const signal = new AbortController().signal;
        await fixture.expect("seeded", signal);
        await fixture.command("replace", "replaced", signal);
        await fixture.command("verify", "verified", signal);
    }
    finally
    {
        await fixture.close();
    }
    try
    {
        expect(await Bun.file(restored).text()).toBe("restored");
    }
    finally
    {
        await rm(directory, { recursive: true });
    }
});

test("cancelling observation still lets the clipboard owner restore through EOF", async () =>
{
    const root = resolve("target/test-work");
    await mkdir(root, { recursive: true });
    const directory = await mkdtemp(join(root, "clipboard-cancel-"));
    const restored = join(directory, "restored");
    const fixture = new NativeFixture([
        "bun",
        "-e",
        `
        console.log("seeded");
        await Bun.stdin.text();
        await Bun.write(process.argv[1], "restored");
        `,
        restored
    ], "clipboard");
    const cancellation = new AbortController();
    try
    {
        await fixture.expect("seeded", cancellation.signal);
        const pending = fixture.command("verify", "verified", cancellation.signal);
        cancellation.abort(new Error("test interruption"));
        await expect(pending).rejects.toThrow();
    }
    finally
    {
        await fixture.close();
    }
    try
    {
        expect(await Bun.file(restored).text()).toBe("restored");
    }
    finally
    {
        await rm(directory, { recursive: true });
    }
});

test("restoration errors are reported instead of treating process exit as success", async () =>
{
    const fixture = new NativeFixture([
        "bun",
        "-e",
        `console.log("seeded"); await Bun.stdin.text(); console.error("restore rejected"); process.exit(23);`
    ], "clipboard");
    await fixture.expect("seeded", new AbortController().signal);
    const result = await fixture.close().catch((error: unknown) => error);
    expect(result).toBeInstanceOf(AggregateError);
    expect((result as AggregateError).errors.map(String).join("\n")).toContain("restore rejected");
});
