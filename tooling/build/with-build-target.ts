import { lstat, mkdir } from "node:fs/promises";
import { join } from "node:path";

import { withBuildLock } from "./with-build-lock.ts";
import { runProcessTree } from "../quality/process-tree.ts";

import type { BuildCommand } from "./BuildCommand.ts";

/**
 * Owns a staging slot while reusing the shared Cargo cache.
 *
 * @param root Repository root
 * @param slot Fixed staging slot; Cargo pipelines share one exclusive lock
 * @param execute Build work; use the provided runner for every child process
 */
export async function withBuildTarget(
    root: string,
    slot: "build" | "check" | "computer-use" | "dev" | "test",
    execute: (
        target: string,
        run: BuildCommand
    ) => Promise<void>
): Promise<void>
{
    // Cargo's own lock ends before staging, packaging and launching. Keep the
    // entire pipeline exclusive so another command cannot replace its executable.
    await withBuildLock(root, slot === "test" ? "test" : "cargo", async () =>
    {
        const target = join(root, "target", `${slot}-work`);
        const cargoTarget = join(root, "target/cargo");
        await mkdir(cargoTarget, { recursive: true });
        if ((await lstat(cargoTarget)).isSymbolicLink())
        {
            throw new Error("The Cargo cache must not be a symbolic link.");
        }
        await mkdir(target, { recursive: true });
        if ((await lstat(target)).isSymbolicLink())
        {
            throw new Error("A build target must not be a symbolic link.");
        }
        const environment = {
            ...process.env,
            CARGO_TARGET_DIR: cargoTarget,
            ...(process.platform === "darwin" ? { MACOSX_DEPLOYMENT_TARGET: "13.0" } : {})
        };
        const cancellation = new AbortController();
        const interrupt = () =>
        {
            cancellation.abort("SIGINT");
        };
        const terminate = () =>
        {
            cancellation.abort("SIGTERM");
        };
        process.on("SIGINT", interrupt);
        process.on("SIGTERM", terminate);
        try
        {
            await execute(target, async (command, cwd = root, overrides = {}) =>
            {
                cancellation.signal.throwIfAborted();
                const child = await runProcessTree(
                    command,
                    cwd,
                    { ...environment, ...overrides },
                    cancellation.signal
                );
                if (cancellation.signal.aborted || child.exitCode !== 0)
                {
                    throw new Error(
                        `${command.join(" ")} failed: ${
                            cancellation.signal.reason ?? child.signalCode ?? child.exitCode
                        }`
                    );
                }
            });
            cancellation.signal.throwIfAborted();
        }
        finally
        {
            process.removeListener("SIGINT", interrupt);
            process.removeListener("SIGTERM", terminate);
        }
    });
}
