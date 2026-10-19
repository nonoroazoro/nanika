import { createInterface } from "node:readline";
import { Readable } from "node:stream";

/**
 * A Rust supervisor retains the process group or Job until every descendant exits.
 * Its pipe also revokes ownership when the Bun runner crashes.
 */
export class OwnedProcess
{
    pid = 0;
    exitCode: number | null = null;
    signalCode: number | null = null;
    readonly exited: Promise<void>;
    private readonly _supervisor: Bun.Subprocess<"pipe", "pipe", "inherit">;
    private readonly _ready = Promise.withResolvers<undefined>();
    private _finished = false;

    private constructor(
        runner: string,
        command: string[],
        cwd: string,
        environment: Record<string, string | undefined>,
        managed: boolean,
        log?: Bun.BunFile
    )
    {
        this._supervisor = Bun.spawn([runner, managed ? "managed" : "closed", ...command], {
            cwd,
            env: environment,
            stdin: "pipe",
            stdout: "pipe",
            stderr: log ?? "inherit",
            windowsHide: true,
            detached: process.platform === "darwin"
        });
        this.exited = this._observe();
        // Observation starts at spawn, before callers await readiness or completion.
        void this.exited.catch(() =>
        {});
    }

    static async spawn(
        runner: string,
        command: string[],
        cwd: string,
        environment: Record<string, string | undefined>,
        managed = false,
        log?: Bun.BunFile
    ): Promise<OwnedProcess>
    {
        const process = new OwnedProcess(runner, command, cwd, environment, managed, log);
        await process._ready.promise;
        return process;
    }

    async control(command: "close" | "interrupt" | "shutdown" | "stop"): Promise<void>
    {
        if (this._finished)
        {
            return;
        }
        try
        {
            await this._supervisor.stdin.write(`${command}\n`);
            await this._supervisor.stdin.flush();
        }
        catch (error)
        {
            // A command can finish between checking state and writing control.
            await this.exited;
            if (!this._finished)
            {
                throw error;
            }
        }
    }

    private async _observe(): Promise<void>
    {
        try
        {
            const lines = createInterface({ input: Readable.from(this._supervisor.stdout) });
            let result = false;
            for await (const line of lines)
            {
                const event = JSON.parse(line) as {
                    exitCode?: number | null;
                    pid?: number;
                    signalCode?: number | null;
                };
                if (typeof event.pid === "number" && Number.isSafeInteger(event.pid) && event.pid > 0 && !this.pid)
                {
                    this.pid = event.pid;
                    this._ready.resolve(undefined);
                }
                else if (this.pid && !result && "exitCode" in event && "signalCode" in event)
                {
                    this.exitCode = event.exitCode ?? null;
                    this.signalCode = event.signalCode ?? null;
                    result = true;
                }
                else
                {
                    throw new Error("Invalid process supervisor response");
                }
            }
            const code = await this._supervisor.exited;
            if (code !== 0 || !result)
            {
                throw new Error(`Process supervisor failed (${code}); process cleanup was not verified`);
            }
        }
        catch (error)
        {
            // EOF asks the supervisor to retire the child even after a protocol failure.
            await Promise.resolve(this._supervisor.stdin.end()).catch(() =>
            {});
            await this._supervisor.exited;
            this._ready.reject(error);
            throw error;
        }
        finally
        {
            this._finished = true;
            await Promise.resolve(this._supervisor.stdin.end()).catch(() =>
            {});
        }
    }
}
