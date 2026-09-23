import { Database } from "bun:sqlite";
import { existsSync } from "node:fs";
import { lstat, mkdir } from "node:fs/promises";
import { join } from "node:path";

export const buildSlotLocks = ".build-locks";

/**
 * Holds an OS-backed exclusive lock for a build pipeline.
 * Manual cleanup may remove the lock files only after builds have stopped.
 *
 * @param root Repository root
 * @param slot Cargo pipeline or standalone tooling tests
 * @param execute Work protected until completion
 */
export async function withBuildLock(
    root: string,
    slot: "cargo" | "test",
    execute: () => Promise<void>
): Promise<void>
{
    if (process.platform !== "darwin" && process.platform !== "win32")
    {
        throw new Error(`Unsupported build platform: ${process.platform}`);
    }
    const target = join(root, "target");
    await mkdir(target, { recursive: true });
    const directory = join(target, buildSlotLocks);
    if ((await lstat(target)).isSymbolicLink())
    {
        throw new Error("Build output must not be a symbolic link.");
    }
    await mkdir(directory, { recursive: true });
    const path = join(directory, `${slot}.sqlite`);
    if ((await lstat(directory)).isSymbolicLink() || (existsSync(path) && (await lstat(path)).isSymbolicLink()))
    {
        throw new Error("Build output and its lock must not be symbolic links.");
    }
    const lock = new Database(path);
    try
    {
        try
        {
            // This database is only a lock and never commits data or needs disk journals.
            lock.run("PRAGMA journal_mode=MEMORY");
            lock.run("BEGIN EXCLUSIVE");
        }
        catch (error)
        {
            if (
                !(error instanceof Error && "code" in error
                    && (error.code === "SQLITE_BUSY" || error.code === "SQLITE_LOCKED"))
            )
            {
                throw error;
            }
            throw new Error("Build output is in use. Stop the active command before retrying.", { cause: error });
        }
        await execute();
    }
    finally
    {
        lock.close();
    }
}
