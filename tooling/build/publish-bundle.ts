import { existsSync } from "node:fs";
import { mkdir, rename, rm } from "node:fs/promises";
import { join } from "node:path";

/**
 * Replaces the one retained bundle only after packaging has succeeded.
 * A failed replacement restores the previous bundle; failed compensation stays visible.
 *
 * @param source Completed bundle directory in the Cargo target
 * @param destination Profile directory for the retained bundle
 */
export async function publishBundle(source: string, destination: string): Promise<void>
{
    await mkdir(destination, { recursive: true });
    const current = join(destination, "bundle");
    const previous = join(destination, "bundle.previous");
    if (existsSync(previous))
    {
        throw new Error(`An interrupted bundle replacement remains at ${previous}. Run just clean before building.`);
    }
    let replaced = false;
    try
    {
        await rename(current, previous);
        replaced = true;
    }
    catch (error)
    {
        if (!(error instanceof Error && "code" in error && error.code === "ENOENT"))
        {
            throw error;
        }
    }
    try
    {
        await rename(source, current);
    }
    catch (error)
    {
        if (replaced)
        {
            try
            {
                await rename(previous, current);
            }
            catch (restoreError)
            {
                throw new AggregateError(
                    [error, restoreError],
                    `Bundle replacement failed; old output remains at ${previous}`,
                    { cause: restoreError }
                );
            }
        }
        throw error;
    }
    if (replaced)
    {
        await rm(previous, { recursive: true });
    }
}
