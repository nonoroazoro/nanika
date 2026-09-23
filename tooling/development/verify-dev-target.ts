import "../runtime.ts";

import { existsSync, rmSync } from "node:fs";
import { fileURLToPath } from "node:url";

import { nanikaProcesses } from "./processes.ts";

if (nanikaProcesses().length !== 0)
{
    throw new Error("Stop the running Nanika instance before starting another development mode.");
}

// Remove old bundles while retaining Cargo's incremental artifacts.
for (const profile of ["debug", "release"])
{
    const bundleDirectory = fileURLToPath(new URL(`../../target/${profile}/bundle`, import.meta.url));
    if (existsSync(bundleDirectory))
    {
        rmSync(bundleDirectory, { recursive: true });
    }
}
