import "../runtime.ts";

import { rm } from "node:fs/promises";
import { isAbsolute, join, parse, resolve } from "node:path";

import { nanikaProcesses } from "./processes.ts";

if (nanikaProcesses().length !== 0)
{
    throw new Error("Stop the running Nanika instance before resetting development state.");
}

let directories: string[];
if (process.platform === "darwin")
{
    const home = _requiredDirectory("HOME");
    directories = [
        join(home, "Library", "Application Support", "Nanika"),
        join(home, "Library", "Caches", "Nanika"),
        // The embedded browser owns these directories outside Nanika's data roots.
        join(home, "Library", "WebKit", "nanika-desktop"),
        join(home, "Library", "Caches", "nanika-desktop")
    ];
}
else if (process.platform === "win32")
{
    const localData = _requiredDirectory("LOCALAPPDATA");
    directories = [join(localData, "Nanika"), join(localData, "app.nanika")];
}
else
{
    throw new Error(`Unsupported development platform: ${process.platform}`);
}

for (const directory of directories)
{
    await rm(directory, { recursive: true, force: true });
}

function _requiredDirectory(name: string): string
{
    const value = process.env[name];
    if (!value?.trim() || !isAbsolute(value))
    {
        throw new Error(`${name} must be an absolute directory path.`);
    }
    const directory = resolve(value);
    if (directory === parse(directory).root)
    {
        throw new Error(`${name} must not be a filesystem root.`);
    }
    return directory;
}
