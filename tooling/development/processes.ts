/**
 * Lists native process IDs so development commands cannot replace a running application.
 */
export function nanikaProcesses(): string[]
{
    if (process.platform === "darwin")
    {
        const result = Bun.spawnSync(["lsof", "-t", "-c", "nanika-desktop"], { stderr: "pipe" });
        if ((result.exitCode !== 0 && result.exitCode !== 1) || result.stderr.length > 0)
        {
            throw new Error(
                `Could not check Nanika processes (${
                    result.signalCode ?? result.exitCode
                }): ${result.stderr.toString()}`
            );
        }
        return result.stdout.toString().trim().split(/\s+/)
            .filter(Boolean);
    }
    if (process.platform === "win32")
    {
        const result = Bun.spawnSync(["tasklist", "/FI", "IMAGENAME eq nanika-desktop.exe", "/FO", "CSV", "/NH"], {
            windowsHide: true,
            stderr: "pipe"
        });
        if (!result.success)
        {
            throw new Error(
                `Could not check Nanika processes (${
                    result.signalCode ?? result.exitCode
                }): ${result.stderr.toString()}`
            );
        }
        return [...result.stdout.toString().matchAll(/^"nanika-desktop\.exe","(\d+)"/gim)]
            .flatMap(match => (match[1] ? [match[1]] : []));
    }
    throw new Error(`Unsupported development platform: ${process.platform}`);
}
