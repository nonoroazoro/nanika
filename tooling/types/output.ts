import { lstat, mkdir, readdir, rename, rm } from "node:fs/promises";
import { dirname, join } from "node:path";

/**
 * Compares complete generated trees or replaces output after generation succeeds.
 *
 * @param source Fresh, caller-owned generation directory
 * @param destination Directory reserved exclusively for generated files
 * @param update Whether to publish instead of checking
 */
export async function publishTypes(source: string, destination: string, update: boolean): Promise<void>
{
    const expected = await _files(source);
    if (expected.length === 0)
    {
        throw new Error("Type generation produced no files.");
    }
    const actual = await _files(destination);
    const differences: string[] = [];
    for (const path of new Set([...expected, ...actual]))
    {
        if (
            !expected.includes(path) || !actual.includes(path)
            || await Bun.file(join(source, path)).text() !== await Bun.file(join(destination, path)).text()
        )
        {
            differences.push(path);
        }
    }
    if (differences.length === 0)
    {
        return;
    }
    if (!update)
    {
        throw new Error(`Generated types are stale: ${differences.sort().join(", ")}. Run bun run types:update.`);
    }
    await mkdir(dirname(destination), { recursive: true });
    // Retain the old tree until the replacement is installed, including on rename failure.
    const backup = `${source}-previous`;
    const exists = await _exists(destination);
    if (exists)
    {
        await rename(destination, backup);
    }
    try
    {
        await rename(source, destination);
    }
    catch (error)
    {
        if (exists)
        {
            await rename(backup, destination);
        }
        throw error;
    }
    if (exists)
    {
        await rm(backup, { recursive: true });
    }
}

async function _files(directory: string): Promise<string[]>
{
    if (!(await _exists(directory)))
    {
        return [];
    }
    if ((await lstat(directory)).isSymbolicLink())
    {
        throw new Error(`Generated directories must not be symbolic links: ${directory}`);
    }
    const files: string[] = [];
    for (const entry of await readdir(directory, { withFileTypes: true }))
    {
        if (entry.isDirectory())
        {
            files.push(...(await _files(join(directory, entry.name))).map(path => `${entry.name}/${path}`));
        }
        else if (entry.isFile() && entry.name.endsWith(".ts"))
        {
            files.push(entry.name);
        }
        else
        {
            throw new Error(`Unexpected generated entry: ${join(directory, entry.name)}`);
        }
    }
    return files.sort();
}

async function _exists(path: string): Promise<boolean>
{
    try
    {
        await lstat(path);
        return true;
    }
    catch (error)
    {
        if (error instanceof Error && "code" in error && error.code === "ENOENT")
        {
            return false;
        }
        throw error;
    }
}
