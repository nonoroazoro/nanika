import { chmod, mkdir, mkdtemp, readdir, rm, stat, utimes } from "node:fs/promises";
import { join, resolve } from "node:path";
import { expect, test } from "vitest";

import { buildExtensions } from "../../build/build-extensions.ts";

test("concurrent builds stage independent copies and repeated builds preserve unchanged sidecars", async () =>
{
    const root = process.env.CARGO_TARGET_DIR ?? resolve(import.meta.dirname, "../../../target/test-work");
    await mkdir(root, { recursive: true });
    const fixture = await mkdtemp(join(root, "extensions-test-"));
    try
    {
        await Promise.all(["dev", "check"].map(async owner =>
        {
            const target = join(fixture, owner);
            const configuration = JSON.parse(
                await buildExtensions(target, "debug", async command =>
                {
                    for (let index = 0; index < command.length; index++)
                    {
                        if (command[index] === "-p")
                        {
                            const suffix = process.platform === "win32" ? ".exe" : "";
                            const binary = join(target, "debug", `${command[index + 1]}${suffix}`);
                            await Bun.write(binary, owner);
                            await chmod(binary, 0o755);
                        }
                    }
                })
            ) as { bundle: { externalBin: string[]; }; };
            expect(configuration.bundle.externalBin.length).toBeGreaterThan(0);
            for (const binary of configuration.bundle.externalBin)
            {
                expect(binary.startsWith(join(target, "sidecars"))).toBe(true);
            }
            const staged = (await readdir(join(target, "sidecars"))).map(name => join(target, "sidecars", name));
            const timestamp = new Date("2000-01-01T00:00:00Z");
            for (const binary of staged)
            {
                await utimes(binary, timestamp, timestamp);
            }
            await buildExtensions(target, "debug", async () =>
            {});
            for (const binary of staged)
            {
                expect((await stat(binary)).mtime.getTime()).toBe(timestamp.getTime());
            }
        }));
        for (const owner of ["dev", "check"])
        {
            const staging = join(fixture, owner, "sidecars");
            for (const name of await readdir(staging))
            {
                const binary = join(staging, name);
                expect(await Bun.file(binary).text()).toBe(owner);
                if (process.platform === "darwin")
                {
                    expect((await stat(binary)).mode & 0o777).toBe(0o755);
                }
            }
        }
    }
    finally
    {
        await rm(fixture, { recursive: true });
    }
});

test("staging replaces changed binaries even when size and modification time match", async () =>
{
    const root = process.env.CARGO_TARGET_DIR ?? resolve(import.meta.dirname, "../../../target/test-work");
    await mkdir(root, { recursive: true });
    const fixture = await mkdtemp(join(root, "extensions-test-"));
    const target = join(fixture, "stage");
    const cargoTarget = join(fixture, "cargo");
    try
    {
        for (const version of ["old", "new"])
        {
            const config = JSON.parse(
                await buildExtensions(target, "debug", async command =>
                {
                    for (let index = 0; index < command.length; index++)
                    {
                        if (command[index] === "-p")
                        {
                            const suffix = process.platform === "win32" ? ".exe" : "";
                            const binary = join(cargoTarget, "debug", `${command[index + 1]}${suffix}`);
                            await Bun.write(binary, version);
                            await utimes(binary, new Date(0), new Date(0));
                        }
                    }
                }, cargoTarget)
            ) as { bundle: { externalBin: string[]; }; };
            expect(config.bundle.externalBin.length).toBeGreaterThan(0);
            for (const name of await readdir(join(target, "sidecars")))
            {
                expect(await Bun.file(join(target, "sidecars", name)).text()).toBe(version);
                await utimes(join(target, "sidecars", name), new Date(0), new Date(0));
            }
        }
        await expect(buildExtensions(target, "debug", async () =>
        {
            throw new Error("compilation failed");
        }, cargoTarget)).rejects.toThrow("compilation failed");
    }
    finally
    {
        await rm(fixture, { recursive: true });
    }
});
