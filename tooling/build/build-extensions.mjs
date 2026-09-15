import { spawnSync } from "node:child_process";

import { extensionNames } from "./extension-binaries.mjs";

const profile = process.argv[2];
if (profile !== "debug" && profile !== "release") {
    throw new Error("usage: build-extensions.mjs <debug|release>");
}

const arguments_ = ["build", "--locked"];
if (profile === "release") {
    arguments_.push("--release");
}
for (const name of extensionNames) {
    arguments_.push("-p", name);
}

const environment = { ...process.env };
if (process.platform === "darwin" && !environment.MACOSX_DEPLOYMENT_TARGET) {
    environment.MACOSX_DEPLOYMENT_TARGET = "13.0";
}

const result = spawnSync("cargo", arguments_, { env: environment, stdio: "inherit" });
if (result.error) {
    throw result.error;
}
if (result.status !== 0) {
    process.exit(result.status ?? 1);
}
