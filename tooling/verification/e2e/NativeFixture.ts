import { createInterface } from "node:readline";
import { Readable } from "node:stream";

/**
 * Owns a native fixture until EOF restores the OS state it borrowed.
 */
export class NativeFixture
{
    private readonly _child: Bun.Subprocess<"pipe", "pipe", "pipe">;
    private readonly _lines: AsyncIterableIterator<string>;
    private readonly _stderr: Promise<string>;
    private readonly _name: string;

    constructor(command: string[], name: string)
    {
        this._name = name;
        this._child = Bun.spawn(command, {
            stdin: "pipe",
            stdout: "pipe",
            stderr: "pipe",
            windowsHide: true,
            // Bun's Windows child Job would kill restoration on owner exit.
            // Detach from that Job; the outer Rust workflow Job still contains us.
            detached: process.platform === "win32"
        });
        this._lines = createInterface({ input: Readable.from(this._child.stdout) })[Symbol.asyncIterator]();
        this._stderr = new Response(this._child.stderr).text();
    }

    async expect(reply: string, signal: AbortSignal): Promise<void>
    {
        await this._reply(reply, signal);
    }

    async command(command: string, reply: string, signal: AbortSignal): Promise<void>
    {
        await this._child.stdin.write(`${command}\n`);
        await this._child.stdin.flush();
        await this._reply(reply, signal);
    }

    async close(): Promise<void>
    {
        const errors: unknown[] = [];
        try
        {
            await this._child.stdin.end();
        }
        catch (error)
        {
            errors.push(error);
        }
        let deadline: ReturnType<typeof setTimeout> | undefined;
        try
        {
            await Promise.race([
                this._child.exited,
                new Promise<never>((_resolve, reject) =>
                {
                    deadline = setTimeout(() =>
                    {
                        reject(new Error(`${this._name} restoration timed out`));
                    }, 10000);
                })
            ]);
        }
        catch (error)
        {
            errors.push(error);
            this._child.kill("SIGKILL");
            await this._child.exited;
        }
        finally
        {
            clearTimeout(deadline);
            await this._lines.return?.();
        }
        const stderr = await this._stderr;
        if (this._child.exitCode !== 0)
        {
            errors.push(new Error(`${this._name} restoration failed (${this._child.exitCode}): ${stderr}`));
        }
        if (errors.length)
        {
            throw new AggregateError(errors, `Could not restore ${this._name}`);
        }
    }

    private async _reply(expected: string, cancellation: AbortSignal): Promise<void>
    {
        const signal = AbortSignal.any([cancellation, AbortSignal.timeout(10000)]);
        signal.throwIfAborted();
        let abort = () =>
        {};
        const interrupted = new Promise<never>((_resolve, reject) =>
        {
            abort = () =>
            {
                reject(new Error(`${this._name} fixture interrupted`, { cause: signal.reason }));
            };
            signal.addEventListener("abort", abort, { once: true });
        });
        try
        {
            const reply = await Promise.race([this._lines.next(), interrupted]);
            if (reply.value !== expected)
            {
                throw new Error(`${this._name} fixture expected ${expected}, received ${reply.value ?? "EOF"}`);
            }
        }
        finally
        {
            signal.removeEventListener("abort", abort);
        }
    }
}
