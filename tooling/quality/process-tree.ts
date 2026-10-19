import { createHash } from "node:crypto";
import { copyFile, mkdir, readFile, rename } from "node:fs/promises";
import { join } from "node:path";
import { fileURLToPath } from "node:url";

import { OwnedProcess } from "./OwnedProcess.ts";
import { withBuildLock } from "../build/with-build-lock.ts";

let preparedRunner: Promise<string> | null = null;

/**
 * Run immutable supervisor artifacts so independent workflows can rebuild safely.
 */
export async function processRunner(): Promise<string>
{
    preparedRunner = preparedRunner ?? _prepareRunner();
    return preparedRunner;
}

/**
 * Run one command until its entire owned tree exits.
 * @param command Executable and arguments
 * @param cwd Working directory
 * @param environment Command environment
 * @param cancellation Workflow cancellation
 * @param shutdown Cooperative stdin protocol for runners that restore OS state
 */
export async function runProcessTree(
    command: string[],
    cwd: string,
    environment: Record<string, string | undefined>,
    cancellation: AbortSignal,
    shutdown?: "cooperative"
): Promise<OwnedProcess>
{
    const runner = await processRunner();
    cancellation.throwIfAborted();
    const child = await OwnedProcess.spawn(
        runner,
        command,
        cwd,
        { ...environment, NANIKA_PROCESS_RUNNER: runner },
        shutdown === "cooperative"
    );
    const interrupted = Promise.withResolvers<undefined>();
    const cancel = () =>
    {
        interrupted.resolve(undefined);
    };
    cancellation.addEventListener("abort", cancel, { once: true });
    try
    {
        if (cancellation.aborted)
        {
            cancel();
        }
        await Promise.race([child.exited, interrupted.promise]);
        if (cancellation.aborted)
        {
            let control: "interrupt" | "shutdown" | "stop" = "stop";
            if (shutdown === "cooperative")
            {
                control = "shutdown";
            }
            else if (cancellation.reason === "SIGINT")
            {
                control = "interrupt";
            }
            await child.control(control);
        }
        await child.exited;
        return child;
    }
    finally
    {
        cancellation.removeEventListener("abort", cancel);
    }
}

/**
 * Stop and await an owned tree, including when its leader has already exited.
 * @param child Owned command
 * @param signal Requested interruption
 */
export async function stopProcessTree(child: OwnedProcess, signal: "SIGINT" | "SIGTERM"): Promise<void>
{
    await child.control(signal === "SIGINT" ? "interrupt" : "stop");
    await child.exited;
}

async function _prepareRunner(): Promise<string>
{
    if (process.env.NANIKA_PROCESS_RUNNER)
    {
        return process.env.NANIKA_PROCESS_RUNNER;
    }
    const root = fileURLToPath(new URL("../../", import.meta.url));
    let runner = "";
    await withBuildLock(root, "supervisor", async () =>
    {
        // Bootstrap is the only command that cannot be launched by the binary it builds.
        const build = Bun.spawn(["cargo", "build", "-p", "nanika-process", "--locked"], {
            cwd: root,
            env: {
                ...process.env,
                CARGO_TARGET_DIR: join(root, "target/cargo"),
                ...(process.platform === "darwin" ? { MACOSX_DEPLOYMENT_TARGET: "13.0" } : {})
            },
            windowsHide: true,
            stdin: "ignore",
            stdout: "inherit",
            stderr: "inherit"
        });
        if (await build.exited !== 0)
        {
            throw new Error("Could not build the process supervisor");
        }
        const name = `nanika-process${process.platform === "win32" ? ".exe" : ""}`;
        const source = join(root, "target/cargo/debug", name);
        const digest = createHash("sha256").update(await readFile(source)).digest("hex");
        const directory = join(root, "target/process-runners", digest);
        runner = join(directory, name);
        if (!await Bun.file(runner).exists())
        {
            await mkdir(directory, { recursive: true });
            await copyFile(source, `${runner}.tmp`);
            await rename(`${runner}.tmp`, runner);
        }
    });
    return runner;
}
