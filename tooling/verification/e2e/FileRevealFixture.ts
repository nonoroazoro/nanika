import { dirname } from "node:path";

/**
 * Owns a Finder or Explorer window used to verify a real application reveal.
 */
export class FileRevealFixture
{
    private readonly _path: string;
    private readonly _existing: string;

    private constructor(path: string, existing: string)
    {
        this._path = path;
        this._existing = existing;
    }

    static async open(path: string): Promise<FileRevealFixture>
    {
        const existing = await _script(
            "return id of every Finder window",
            "(@($shell.Windows()) | ForEach-Object { $_.HWND }) -join ', '",
            []
        );
        if (!/^(?:\d+(?:, \d+)*)?$/.test(existing))
        {
            throw new Error("File manager returned invalid window identities");
        }
        const fixture = new FileRevealFixture(path, existing);
        try
        {
            // Give reveal an owned destination instead of borrowing a user's window.
            await _script(
                "make new Finder window to (POSIX file (item 1 of argv) as alias)",
                `
                $shell.Explore($arguments[0])
                $deadline = [DateTime]::UtcNow.AddSeconds(5)
                while (-not (@($shell.Windows()) | Where-Object {
                    @(${existing}) -notcontains $_.HWND -and $_.Document.Folder.Self.Path -ieq $arguments[0]
                })) {
                    if ([DateTime]::UtcNow -ge $deadline) { throw 'Owned Explorer window did not open' }
                    Start-Sleep -Milliseconds 50
                }
                `,
                [dirname(path)]
            );
            return fixture;
        }
        catch (error)
        {
            await fixture.close();
            throw error;
        }
    }

    async selected(): Promise<boolean>
    {
        return await _script(
            `
            if (count Finder windows) is 0 then return false
            if (id of front Finder window) is in {${this._existing}} then return false
            if (count (get selection)) is not 1 then return false
            return (POSIX path of ((item 1 of (get selection)) as alias)) is (item 1 of argv)
        `,
            `
            foreach ($window in @($shell.Windows())) {
                if (@(${this._existing}) -contains $window.HWND) { continue }
                if ($window.Document.Folder.Self.Path -ine $arguments[1]) { continue }
                $selection = $window.Document.SelectedItems()
                if ($selection.Count -eq 1 -and $selection.Item(0).Path -ieq $arguments[0]) {
                    return 'true'
                }
            }
            return 'false'
            `,
            [process.platform === "darwin" ? `${this._path}/` : this._path, dirname(this._path)]
        ) === "true";
    }

    async close(): Promise<void>
    {
        const remaining = await _script(
            `
            repeat with candidate in (get every Finder window)
                if (id of candidate) is not in {${this._existing}} then
                    if (POSIX path of ((target of candidate) as alias)) is (item 1 of argv) then
                        close candidate
                    end if
                end if
            end repeat
            repeat with candidate in (get every Finder window)
                if (id of candidate) is not in {${this._existing}} then
                    if (POSIX path of ((target of candidate) as alias)) is (item 1 of argv) then return "remaining"
                end if
            end repeat
            return "closed"
        `,
            `
            function OwnedWindows {
                @($shell.Windows()) | Where-Object {
                    @(${this._existing}) -notcontains $_.HWND -and $_.Document.Folder.Self.Path -ieq $arguments[0]
                }
            }
            foreach ($window in @(OwnedWindows)) { $window.Quit() }
            $deadline = [DateTime]::UtcNow.AddSeconds(5)
            while (@(OwnedWindows).Count -ne 0) {
                if ([DateTime]::UtcNow -ge $deadline) { throw 'Owned Explorer windows did not close' }
                Start-Sleep -Milliseconds 50
            }
            return 'closed'
            `,
            [dirname(this._path) + (process.platform === "darwin" ? "/" : "")]
        );
        if (remaining !== "closed")
        {
            throw new Error("Owned file manager windows did not close");
        }
    }
}

async function _script(finder: string, explorer: string, args: string[]): Promise<string>
{
    const windows = process.platform === "win32";
    const script = windows
        ? `$ErrorActionPreference = 'Stop'
            $arguments = @(ConvertFrom-Json $env:NANIKA_E2E_REVEAL_ARGUMENTS)
            $shell = New-Object -ComObject Shell.Application
            ${explorer}`
        : `on run argv
            with timeout of 5 seconds
                tell application "Finder"
                    ${finder}
                end tell
            end timeout
        end run`;
    const command = windows
        ? [
            "powershell.exe",
            "-NoLogo",
            "-NoProfile",
            "-NonInteractive",
            "-EncodedCommand",
            Buffer.from(script, "utf16le").toString("base64")
        ]
        : ["/usr/bin/osascript", "-", ...args];
    const child = Bun.spawn(command, {
        stdin: "pipe",
        stdout: "pipe",
        stderr: "pipe",
        windowsHide: true,
        timeout: 10000,
        env: { ...process.env, NANIKA_E2E_REVEAL_ARGUMENTS: JSON.stringify(args) }
    });
    if (!windows)
    {
        await child.stdin.write(script);
    }
    await child.stdin.end();
    const [code, stdout, stderr] = await Promise.all([
        child.exited,
        new Response(child.stdout).text(),
        new Response(child.stderr).text()
    ]);
    if (code !== 0)
    {
        throw new Error(`File reveal fixture failed (${code}): ${stderr.trim()}`);
    }
    return stdout.trim();
}
