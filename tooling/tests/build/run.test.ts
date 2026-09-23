import { mkdir, mkdtemp, rm } from "node:fs/promises";
import { join, resolve } from "node:path";
import { expect, test } from "vitest";

test.each(["dev", "build", "computer-use"] as const)(
    "%s rebuilds changed Rust source before launch or publication, and stops on compilation failure",
    async mode =>
    {
        if (mode === "computer-use" && process.platform !== "darwin")
        {
            return;
        }
        const fixture = await _fixture();
        try
        {
            for (const version of ["old", "new"])
            {
                await Bun.write(join(fixture, "src/main.rs"), `fn main() { println!("${version}"); }`);
                await Bun.write(join(fixture, "target/cargo/debug/bundle/stale"), "old bundle");
                await Bun.write(join(fixture, "target/cargo/release/bundle/stale"), "old bundle");
                const result = await _run(fixture, mode);
                expect(result.exitCode, result.stderr).toBe(0);
                expect(await Bun.file(join(fixture, "observed")).text()).toBe(version);
                if (mode !== "dev")
                {
                    const profile = mode === "build" ? "release" : "debug";
                    expect(await Bun.file(join(fixture, `target/${profile}/bundle/app`)).text()).toBe(version);
                    expect(await Bun.file(join(fixture, `target/${profile}/bundle/stale`)).exists()).toBe(false);
                }
                if (mode === "computer-use")
                {
                    expect(await Bun.file(join(fixture, "launched")).text()).toBe(version);
                }
            }
            await rm(join(fixture, "observed"));
            await rm(join(fixture, "launched"), { force: true });
            await Bun.write(join(fixture, "src/main.rs"), "invalid Rust source");
            const failed = await _run(fixture, mode);
            expect(failed.exitCode).not.toBe(0);
            expect(failed.stderr).toContain("failed");
            expect(await Bun.file(join(fixture, "observed")).exists()).toBe(false);
            expect(await Bun.file(join(fixture, "launched")).exists()).toBe(false);
            if (mode !== "dev")
            {
                const profile = mode === "build" ? "release" : "debug";
                expect(await Bun.file(join(fixture, `target/${profile}/bundle/app`)).text()).toBe("new");
            }
            await Bun.write(join(fixture, "src/main.rs"), 'fn main() { println!("new"); }');
            await Bun.write(join(fixture, "fail-desktop"), "fail");
            const desktopFailure = await _run(fixture, mode);
            expect(desktopFailure.exitCode).not.toBe(0);
            expect(desktopFailure.stderr).toContain("desktop build failed");
            expect(await Bun.file(join(fixture, "launched")).exists()).toBe(false);
            if (mode !== "dev")
            {
                const profile = mode === "build" ? "release" : "debug";
                expect(await Bun.file(join(fixture, `target/${profile}/bundle/app`)).text()).toBe("new");
            }
            if (mode !== "build")
            {
                await Bun.write(join(fixture, "running"), "old instance");
                expect((await _run(fixture, mode)).stderr).toContain("Stop the running Nanika instance");
            }
        }
        finally
        {
            await rm(fixture, { recursive: true });
        }
    },
    30_000
);

async function _fixture(): Promise<string>
{
    const repository = resolve(import.meta.dirname, "../../..");
    const root = process.env.CARGO_TARGET_DIR ?? join(repository, "target/test-work");
    await mkdir(root, { recursive: true });
    const fixture = await mkdtemp(join(root, "run-test-"));
    for (
        const file of [
            "tooling/runtime.ts",
            "tooling/build/run.ts",
            "tooling/build/build-extensions.ts",
            "tooling/build/publish-bundle.ts",
            "tooling/build/with-build-target.ts",
            "tooling/build/with-build-lock.ts",
            "tooling/quality/process-tree.ts"
        ]
    )
    {
        await Bun.write(join(fixture, file), Bun.file(join(repository, file)));
    }
    await Bun.write(
        join(fixture, "package.json"),
        JSON.stringify({
            packageManager: `bun@${Bun.version}`,
            scripts: { tauri: "bun tauri.ts" }
        })
    );
    await Bun.write(
        join(fixture, "Cargo.toml"),
        '[package]\nname="freshness-extension"\nversion="0.1.0"\nedition="2024"\n[workspace]\n'
    );
    await Bun.write(
        join(fixture, "Cargo.lock"),
        'version = 4\n[[package]]\nname = "freshness-extension"\nversion = "0.1.0"\n'
    );
    await Bun.write(
        join(fixture, "apps/desktop/shell/tauri.conf.json"),
        JSON.stringify({ bundle: { externalBin: ["freshness-extension"] } })
    );
    await Bun.write(
        join(fixture, "tooling/development/processes.ts"),
        `
        export function nanikaProcesses() {
            return Bun.file(new URL('../../running', import.meta.url)).size > 0 ? ['123'] : [];
        }
    `
    );
    await Bun.write(
        join(fixture, "tooling/development/run-dev.ts"),
        `
        import { join } from 'node:path';
        export async function runDev(root, config, run) {
            await run([process.execPath, 'run', 'tauri', 'dev', '--config', config], join(root, 'apps/desktop'));
        }
    `
    );
    await Bun.write(
        join(fixture, "tooling/development/launch-computer-use-app.ts"),
        `
        const version = await Bun.file(new URL('../../target/debug/bundle/app', import.meta.url)).text();
        await Bun.write(new URL('../../launched', import.meta.url), version);
    `
    );
    // The fixture uses real Cargo and staging; this CLI stand-in observes the
    // executable handed to Tauri without starting a GUI inside the test suite.
    await Bun.write(
        join(fixture, "tauri.ts"),
        `
        import { join, dirname } from 'node:path';
        import { readdir } from 'node:fs/promises';
        if (await Bun.file(new URL('./fail-desktop', import.meta.url)).exists()) throw new Error('desktop build failed');
        const args = process.argv.slice(2);
        const config = JSON.parse(args[args.indexOf('--config') + 1]);
        const base = config.bundle.externalBin[0];
        const directory = dirname(base);
        const binary = (await readdir(directory)).find(name => name.startsWith('freshness-extension-'));
        const child = Bun.spawn([join(directory, binary)], { stdout: 'pipe', stderr: 'pipe' });
        const version = (await new Response(child.stdout).text()).trim();
        if (await child.exited !== 0) throw new Error('sidecar failed');
        await Bun.write(new URL('./observed', import.meta.url), version);
        if (args[0] === 'build') {
            const profile = args.includes('--debug') ? 'debug' : 'release';
            await Bun.write(join(process.env.CARGO_TARGET_DIR, profile, 'bundle/app'), version);
        }
    `
    );
    return fixture;
}

async function _run(root: string, mode: string): Promise<{ exitCode: number; stderr: string; }>
{
    const child = Bun.spawn([process.execPath, "tooling/build/run.ts", mode], {
        cwd: root,
        stdout: "pipe",
        stderr: "pipe"
    });
    const [exitCode, stderr] = await Promise.all([
        child.exited,
        new Response(child.stderr).text(),
        new Response(child.stdout).text()
    ]);
    return { exitCode, stderr };
}
