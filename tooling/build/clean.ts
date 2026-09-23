import "../runtime.ts";

import { rm } from "node:fs/promises";

// Manual cleanup: stop builds and the development app before running this script.
await rm(new URL("../../target", import.meta.url), { recursive: true, force: true });
console.log("Removed target.");
