import { spawnSync } from "node:child_process";

import { extensionNames } from "./extension-binaries.ts";

const profile = process.argv[2];
if (profile !== "debug" && profile !== "release")
{
    throw new Error("usage: build-extensions.ts <debug|release>");
}

const cargoArguments = ["build", "--locked"];
if (profile === "release")
{
    cargoArguments.push("--release");
}
for (const name of extensionNames)
{
    cargoArguments.push("-p", name);
}

const environment = { ...process.env };
if (process.platform === "darwin" && !environment.MACOSX_DEPLOYMENT_TARGET)
{
    environment.MACOSX_DEPLOYMENT_TARGET = "13.0";
}

const result = spawnSync("cargo", cargoArguments, { env: environment, stdio: "inherit" });
if (result.error)
{
    throw result.error;
}
if (result.status !== 0)
{
    process.exit(result.status ?? 1);
}
