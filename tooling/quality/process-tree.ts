/**
 * Runs a validation command independently of the terminal's process group.
 * macOS cancellation signals the owned group and waits until it is absent.
 * Windows cancellation uses taskkill /T /F and waits for taskkill and the child.
 * A termination failure rejects without claiming that descendants have exited.
 *
 * @param command Executable and arguments
 * @param cwd Command working directory
 * @param environment Explicit command environment
 * @param cancellation Cancellation shared by the validation pipeline
 */
export async function runProcessTree(
    command: string[],
    cwd: string,
    environment: Record<string, string | undefined>,
    cancellation: AbortSignal
): Promise<Bun.Subprocess>
{
    cancellation.throwIfAborted();
    if (process.platform !== "darwin" && process.platform !== "win32")
    {
        throw new Error(`Unsupported validation platform: ${process.platform}`);
    }
    const child = Bun.spawn(command, {
        cwd,
        env: environment,
        detached: true,
        stdin: "ignore",
        stdout: "inherit",
        stderr: "inherit"
    });
    const stopped = Promise.withResolvers<undefined>();
    const cancel = () =>
    {
        void _terminate(child, cancellation.reason === "SIGINT" ? "SIGINT" : "SIGTERM")
            .then(() =>
            {
                stopped.resolve(undefined);
            }, stopped.reject);
    };
    cancellation.addEventListener("abort", cancel, { once: true });
    try
    {
        // A failed termination must surface even when the child is still running.
        await Promise.race([child.exited, stopped.promise]);
        if (cancellation.aborted)
        {
            await stopped.promise;
        }
        return child;
    }
    finally
    {
        cancellation.removeEventListener("abort", cancel);
    }
}

async function _terminate(child: Bun.Subprocess, signal: "SIGINT" | "SIGTERM"): Promise<void>
{
    if (process.platform === "darwin")
    {
        _signalGroup(child.pid, signal);
        await child.exited;
        // The group leader can exit before a descendant finishes handling the signal.
        while (_groupExists(child.pid))
        {
            await Bun.sleep(10);
        }
    }
    else if (process.platform === "win32")
    {
        const termination = Bun.spawn(["taskkill", "/PID", String(child.pid), "/T", "/F"], {
            stdin: "ignore",
            stdout: "pipe",
            stderr: "pipe"
        });
        const [exitCode, stdout, stderr] = await Promise.all([
            termination.exited,
            new Response(termination.stdout).text(),
            new Response(termination.stderr).text()
        ]);
        if (exitCode !== 0)
        {
            throw new Error(
                `Could not terminate validation process tree ${child.pid} (${exitCode}): ${stderr}${stdout}`
            );
        }
        await child.exited;
    }
    else
    {
        throw new Error(`Unsupported validation platform: ${process.platform}`);
    }
}

function _signalGroup(pid: number, signal: "SIGINT" | "SIGTERM"): void
{
    try
    {
        process.kill(-pid, signal);
    }
    catch (error)
    {
        if (error instanceof Error && "code" in error && error.code === "ESRCH")
        {
            return;
        }
        throw error;
    }
}

function _groupExists(pid: number): boolean
{
    // A descendant can be reparented after the group leader exits. Inspect group
    // membership without requiring permission to send another signal to it.
    const result = Bun.spawnSync(["lsof", "-a", "-g", String(pid), "-d", "cwd", "-t"], { stderr: "pipe" });
    if ((result.exitCode !== 0 && result.exitCode !== 1) || result.stderr.length > 0)
    {
        throw new Error(
            `Could not inspect validation process group ${pid} (${result.exitCode}): ${result.stderr.toString()}`
        );
    }
    return result.stdout.length > 0;
}
