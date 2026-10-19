import { readFile } from "node:fs/promises";
import { afterAll, afterEach, beforeAll, beforeEach, describe, expect, it, vi } from "vitest";
import { loadWASM, OnigScanner, OnigString } from "vscode-oniguruma";
import { parseRawGrammar } from "vscode-textmate";

import { languageSamples, syntaxSamples } from "./language-samples";
import { detectSyntax } from "../../src/syntax/detect";
import manifest from "../../src/syntax/grammars/manifest.json";
import { LANGUAGE_SCOPES } from "../../src/syntax/languages";
import { LINE_TIME_MS, TOKEN_BATCH_CHARS } from "../../src/syntax/limits";
import { TOKEN_COLORS } from "../../src/syntax/palette";
import { SyntaxEngine } from "../../src/syntax/SyntaxEngine";

import type { SyntaxLanguage } from "../../src/syntax/SyntaxLanguage";
import type { SyntaxTheme } from "../../src/syntax/SyntaxTheme";

let engine: SyntaxEngine;
beforeAll(async () =>
{
    await loadWASM(
        await readFile(new URL("../../../../../node_modules/vscode-oniguruma/release/onig.wasm", import.meta.url))
    );
    engine = new SyntaxEngine({
        onigLib: Promise.resolve({
            createOnigScanner: patterns => new OnigScanner(patterns),
            createOnigString: text => new OnigString(text)
        }),
        loadGrammar: async scope =>
        {
            const entry = manifest.grammars[scope as keyof typeof manifest.grammars];
            return entry
                ? parseRawGrammar(
                    await readFile(new URL(`../../src/syntax/grammars/${entry.asset}`, import.meta.url), "utf8"),
                    entry.asset
                )
                : null;
        },
        getInjections: scope =>
            Object.entries(manifest.grammars).filter(([, entry]) => (entry.injectTo as string[]).includes(scope)).map((
                [name]
            ) => name)
    });
});
afterAll(() =>
{
    engine?.dispose();
});
beforeEach(() =>
{
    // Grammar/color assertions must not depend on CPU contention in a parallel test run.
    // TextMate's deadline uses Date.now; cooperative yielding keeps its separate performance clock.
    vi.spyOn(Date, "now").mockReturnValue(0);
});
afterEach(() =>
{
    vi.restoreAllMocks();
});

describe("SyntaxEngine", () =>
{
    it.each([
        ["THIRD_PARTY_NOTICES.md", "markdown"],
        ["apps/desktop/frontend/src/styles/global.css", "css"]
    ])("detects and colors copied %s without a filename hint", async (path, language) =>
    {
        const text = await readFile(new URL(`../../../../../${path}`, import.meta.url), "utf8");
        for (const theme of ["light", "dark"] as const)
        {
            const result = await _colorSource(text, null, new AbortController().signal, theme);

            expect(result.language).toBe(language);
            expect(result.ranges.length).toBeGreaterThan(0);
        }
    });

    it("leaves a timed-out line plain without rejecting the document", async () =>
    {
        let time = 0;
        vi.mocked(Date.now).mockImplementation(() =>
        {
            time += LINE_TIME_MS + 1;
            return time;
        });
        const result = await _colorSource(
            'const message = "hello";',
            "javascript",
            new AbortController().signal,
            "light"
        );

        expect(result.ranges.length).toBe(0);
    });

    it("finishes a valid first preview across slow cooperative slices", async () =>
    {
        let time = 0;
        const clock = vi.spyOn(performance, "now").mockImplementation(() =>
        {
            time += 20;
            return time;
        });
        try
        {
            const text = 'const message = "hello";\n'.repeat(20);
            const result = await _colorSource(text, "javascript", new AbortController().signal, "light");
            expect(time).toBeGreaterThan(250);

            expect(_colorAt(result.ranges, text.lastIndexOf("hello"))).toBe("#A31515");
        }
        finally
        {
            clock.mockRestore();
        }
    });

    it("detects and colors the copied ambient module file without a filename hint", async () =>
    {
        const text = await readFile(new URL("./fixtures/modules.txt", import.meta.url), "utf8");
        const result = await _colorSource(text, null, new AbortController().signal, "light");

        expect(result.language).toBe("typescript");
        expect(_colorAt(result.ranges, text.indexOf("declare"))).toBe("#0000FF");
        expect(_colorAt(result.ranges, text.indexOf("@syntax-preview"))).toBe("#A31515");
        expect(_colorAt(result.ranges, text.lastIndexOf("export"))).toBe("#AF00DB");
    });

    it("detects a copied production Svelte component without a filename", async () =>
    {
        const text = await readFile(
            new URL("../../src/components/extensions/TextPreview.svelte", import.meta.url),
            "utf8"
        );
        const result = await _colorSource(text, null, new AbortController().signal, "light");

        expect(result.language).toBe("svelte");
        expect(_colorAt(result.ranges, text.indexOf("import"))).toBe("#AF00DB");
        expect(_colorAt(result.ranges, text.lastIndexOf("style"))).toBe("#800000");
    });

    it("colors the actual Objective-C probe in both themes while keeping embedded JavaScript a string", async () =>
    {
        const text = await readFile(new URL("./fixtures/objective-c.m", import.meta.url), "utf8");
        for (const theme of ["light", "dark"] as const)
        {
            const result = await _colorSource(text, null, new AbortController().signal, theme);
            expect(result.language).toBe("objective-c");

            expect(_colorAt(result.ranges, text.indexOf("const pause="))).toBe(
                theme === "dark" ? "#CE9178" : "#A31515"
            );
            expect(_colorAt(result.ranges, text.indexOf("Owned native"))).toBe(
                theme === "dark" ? "#6A9955" : "#008000"
            );
        }
    });

    it("loads the TOML injection in Markdown code fences", async () =>
    {
        const text = '```toml\n[package]\nname = "hello"\n```';
        const result = await _colorSource(text, null, new AbortController().signal, "light");
        expect(result.language).toBe("markdown");

        expect(_colorAt(result.ranges, text.indexOf("hello"))).toBe("#A31515");
    });

    it("colors Svelte script, template expressions and styles with the official grammar", async () =>
    {
        const text = '<script lang="ts">\nlet count: number = $state(0);\n</script>\n'
            + "{#if count > 0}\n<button onclick={() => count++}>{count}</button>\n{/if}\n"
            + "<style>\nbutton { color: red; }\n</style>";
        const result = await _colorSource(text, null, new AbortController().signal, "light");

        expect(result.language).toBe("svelte");
        expect(_colorAt(result.ranges, text.indexOf("let"))).toBe("#0000FF");
        expect(_colorAt(result.ranges, text.indexOf("number"))).toBe("#267F99");
        expect(_colorAt(result.ranges, text.indexOf("if count"))).toBe("#AF00DB");
        expect(_colorAt(result.ranges, text.indexOf("button onclick"))).toBe("#800000");
        expect(_colorAt(result.ranges, text.indexOf("color:"))).toBeDefined();
        expect(_colorAt(result.ranges, text.lastIndexOf("style"))).toBe("#800000");
    });

    it("switches between official Light+ and Dark+ foregrounds in the same engine", async () =>
    {
        const text = '<script lang="ts">\n// counter\nlet count: number = 42;\n'
            + 'function greet(name: string) { return "hello"; }\n</script>\n<button>{count}</button>';
        for (const theme of ["light", "dark", "light"] as const)
        {
            const result = await _colorSource(text, "svelte", new AbortController().signal, theme);

            const expected = theme === "dark"
                ? {
                    let: "#569CD6",
                    number: "#4EC9B0",
                    greet: "#DCDCAA",
                    return: "#C586C0",
                    hello: "#CE9178",
                    "42": "#B5CEA8",
                    counter: "#6A9955",
                    button: "#569CD6"
                }
                : {
                    let: "#0000FF",
                    number: "#267F99",
                    greet: "#795E26",
                    return: "#AF00DB",
                    hello: "#A31515",
                    "42": "#098658",
                    counter: "#008000",
                    button: "#800000"
                };
            for (const [token, color] of Object.entries(expected))
            {
                expect(_colorAt(result.ranges, text.indexOf(token)), `${theme}: ${token}`).toBe(color);
            }
        }
    });

    it("colors an entire 500-line Markdown document with embedded TypeScript", async () =>
    {
        const block =
            '## Heading\n\nText and **emphasis**.\n\n```typescript\nconst message: string = "hello";\n/* comment\ncontinued */\n```\n\n';
        const text = `${block.repeat(50)}\n## FINAL_HEADING`;
        const result = await _colorSource(text, null, new AbortController().signal, "light");

        expect(result.language).toBe("markdown");
        expect(result.ranges.length / 3).toBeGreaterThan(64);
        expect(_colorAt(result.ranges, text.indexOf("const"))).toBe("#0000FF");
        expect(_colorAt(result.ranges, text.indexOf("hello"))).toBe("#A31515");
        expect(_colorAt(result.ranges, text.indexOf("continued"))).toBe("#008000");
        expect(_colorAt(result.ranges, text.indexOf("FINAL_HEADING"))).toBe("#800000");
    });

    it("keeps state in nested, unclosed fences and preserves Unicode/CRLF offsets", async () =>
    {
        const text =
            '# 😀 Document\r\n\r\n> ```typescript\r\n> /* first 😀\r\n> still a comment */\r\n> const title = "你好😀";';
        const result = await _colorSource(text, "markdown", new AbortController().signal, "light");

        expect(_colorAt(result.ranges, text.indexOf("still"))).toBe("#008000");
        expect(_colorAt(result.ranges, text.indexOf("const"))).toBe("#0000FF");
        expect(_colorAt(result.ranges, text.indexOf("你好"))).toBe("#A31515");
        for (let index = 0; index < result.ranges.length; index += 3)
        {
            expect(result.ranges[index + 1]).toBeLessThanOrEqual(text.length);
            expect(result.ranges[index]).toBeGreaterThanOrEqual(index ? (result.ranges[index - 2] ?? -1) : 0);
        }
    });

    it("requires a representative sample for every admitted language", () =>
    {
        expect(Object.keys(languageSamples).sort()).toEqual(Object.keys(LANGUAGE_SCOPES).sort());
        for (const [language, samples] of Object.entries(languageSamples))
        {
            expect(Object.keys(samples).length, language).toBeGreaterThanOrEqual(2);
        }
    });

    it.each(syntaxSamples)("detects and highlights $language / $name in both themes", async ({ language, text }) =>
    {
        for (const theme of ["light", "dark"] as const)
        {
            // JSONC needs an explicit hint. All other samples exercise detection through tokenization.
            const hint = language === "jsonc" ? language : null;
            const result = await _colorSource(text, hint, new AbortController().signal, theme);
            expect(result.language, theme).toBe(language);

            expect(result.ranges.length, theme).toBeGreaterThan(0);
            expect(result.ranges.length % 3).toBe(0);
            for (let index = 0; index < result.ranges.length; index += 3)
            {
                const start = result.ranges[index] ?? -1;
                const end = result.ranges[index + 1] ?? -1;
                expect(start).toBeGreaterThanOrEqual(index ? (result.ranges[index - 2] ?? -1) : 0);
                expect(end).toBeGreaterThan(start);
                expect(end).toBeLessThanOrEqual(text.length);
                expect(TOKEN_COLORS[result.ranges[index + 2] ?? -1]).toBeDefined();
            }
        }
    });

    it("detects and highlights the repository justfile in both themes", async () =>
    {
        const text = await readFile(new URL("../../../../../justfile", import.meta.url), "utf8");
        for (const theme of ["light", "dark"] as const)
        {
            const result = await _colorSource(text, null, new AbortController().signal, theme);
            expect(result.language).toBe("just");

            expect(_colorAt(result.ranges, text.indexOf("# Start"))).toBe(theme === "dark" ? "#6A9955" : "#008000");
            expect(_colorAt(result.ranges, text.indexOf('"1.56.0"') + 1)).toBe(
                theme === "dark" ? "#CE9178" : "#A31515"
            );
        }
    });

    it("detects and highlights a copied Rust struct and implementation without imports", async () =>
    {
        const text = await readFile(
            new URL("../../../../../engine/runtime/src/ExtensionRuntimeInvocation.rs", import.meta.url),
            "utf8"
        );
        for (const theme of ["light", "dark"] as const)
        {
            const result = await _colorSource(text, null, new AbortController().signal, theme);
            expect(result.language).toBe("rust");

            expect(result.ranges.length).toBeGreaterThan(0);
        }
    });

    it("highlights the copied ESLint configuration as JavaScript without a filename hint", async () =>
    {
        const text = await readFile(new URL("../../../../../eslint.config.ts", import.meta.url), "utf8");
        for (const theme of ["light", "dark"] as const)
        {
            const result = await _colorSource(text, null, new AbortController().signal, theme);
            expect(result.language).toBe("javascript");

            expect(_colorAt(result.ranges, text.indexOf("import"))).toBe(theme === "dark" ? "#C586C0" : "#AF00DB");
            expect(_colorAt(result.ranges, text.indexOf("@typescript-eslint/parser"))).toBe(
                theme === "dark" ? "#CE9178" : "#A31515"
            );
        }
    });

    it("cancels during a yielded tokenization without publishing a partial result", async () =>
    {
        const controller = new AbortController();
        let elapsed = 0;
        vi.spyOn(performance, "now").mockImplementation(() =>
        {
            elapsed += 5;
            return elapsed;
        });
        const result = _colorSource(
            'const title: string = "example";\n'.repeat(1900),
            "typescript",
            controller.signal,
            "light"
        );
        setTimeout(() =>
        {
            controller.abort();
        }, 0);
        await expect(result).rejects.toThrow();
    });
});

function _colorAt(ranges: Uint32Array, offset: number): string | undefined
{
    for (let index = 0; index < ranges.length; index += 3)
    {
        if ((ranges[index] ?? -1) <= offset && (ranges[index + 1] ?? -1) > offset)
        {
            return TOKEN_COLORS[ranges[index + 2] ?? 0];
        }
    }
    return undefined;
}

it.each(
    [
        ["css", ":root {\r\n  /* 😀 a comment\r\n     across batches */\r\n  --accent: red;\r\n}\n"],
        ["markdown", '# Guide\n\n```typescript\n/* across\nlines */\nconst text = "😀";\n```\n\n## End\n'],
        ["rust", 'fn main() {\n    /* comment\n       continued */\n    let value = "😀";\n}\n']
    ] as const
)("preserves %s grammar state and UTF-16 offsets across arbitrary chunk boundaries", async (language, text) =>
{
    const whole = await _colorSource(text, language, new AbortController().signal, "light");
    for (const width of [1, 7, 31])
    {
        const ranges: number[] = [];
        for (let offset = 0; offset < text.length; offset += width)
        {
            const chunk = text.slice(offset, offset + width);
            const result = await engine.append(chunk, language, new AbortController().signal, "light", {
                stream: 100,
                offset,
                final: offset + chunk.length === text.length
            });
            ranges.push(...result.ranges);
        }
        expect(ranges).toEqual([...whole.ranges]);
        engine.release(100);
    }
});

it("colors beyond the former document limit and skips only oversized individual lines", async () =>
{
    const text = `/* header */\n${"x".repeat(9000)}\n${"body { color: red; }\n".repeat(4000)}`;
    let colored = false;
    for (let offset = 0; offset < text.length; offset += 2048)
    {
        const chunk = text.slice(offset, offset + 2048);
        const result = await engine.append(chunk, "css", new AbortController().signal, "light", {
            stream: 101,
            offset,
            final: offset + chunk.length === text.length
        });

        if (offset > 65536 && result.ranges.length)
        {
            colored = true;
        }
        for (let index = 0; index < result.ranges.length; index += 3)
        {
            expect((result.ranges[index] ?? 0) < 13 || (result.ranges[index] ?? 0) >= 9014).toBe(true);
        }
    }
    expect(colored).toBe(true);
    engine.release(101);
});

it("does not commit an aborted batch and releases cursors between repeated previews", async () =>
{
    const controller = new AbortController();
    const text = "body { color: red; }\n".repeat(50);
    const pending = engine.append(text, "css", controller.signal, "light", { stream: 102, offset: 0, final: false });
    controller.abort();
    await expect(pending).rejects.toThrow();
    for (let stream = 102; stream < 122; stream++)
    {
        const result = await engine.append(text, "css", new AbortController().signal, "light", {
            stream,
            offset: 0,
            final: false
        });
        expect(result.ranges.length).toBeGreaterThan(0);
        engine.release(stream);
    }
    await expect(
        engine.append(text, "css", new AbortController().signal, "light", {
            stream: 102,
            offset: text.length,
            final: true
        })
    ).rejects.toThrow("no longer available");
});

async function _colorSource(text: string, hint: SyntaxLanguage | null, signal: AbortSignal, theme: SyntaxTheme)
{
    const language = hint ?? detectSyntax(text);
    if (!language)
    {
        throw new Error("Expected recognizable fixture source");
    }
    const stream = 200;
    const ranges: number[] = [];
    try
    {
        for (let offset = 0; offset < text.length; offset += TOKEN_BATCH_CHARS)
        {
            const chunk = text.slice(offset, offset + TOKEN_BATCH_CHARS);
            const result = await engine.append(chunk, language, signal, theme, {
                stream,
                offset,
                final: offset + chunk.length === text.length
            });
            ranges.push(...result.ranges);
        }
        return { language, ranges: Uint32Array.from(ranges) };
    }
    finally
    {
        engine.release(stream);
    }
}
