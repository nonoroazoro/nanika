import { readFile } from "node:fs/promises";
import { describe, expect, it } from "vitest";

import { syntaxSamples } from "./language-samples";
import { detectSyntax } from "../../src/syntax/detect";
import { DETECTION_LIMIT } from "../../src/syntax/limits";

describe("syntax detection", () =>
{
    it("detects typed YAML mappings without a section header", () =>
    {
        expect(detectSyntax("enabled: true\nretries: 3\n")).toBe("yaml");
    });

    it("recognizes the Objective-C probe before JavaScript strings and header angle brackets", async () =>
    {
        const text = await readFile(new URL("./fixtures/objective-c.m", import.meta.url), "utf8");
        expect(detectSyntax(text)).toBe("objective-c");
        expect(detectSyntax(`/* native source */\n${text}`)).toBe("objective-c");
        expect(detectSyntax('const value = "#import <Foundation/Foundation.h>";')).toBe("javascript");
        expect(detectSyntax(`\`\`\`objc\n${text}\n\`\`\``)).toBe("markdown");
        expect(detectSyntax(JSON.stringify({ source: text }))).toBe("json");
    });

    it("recognizes the outer Markdown document before embedded code", () =>
    {
        expect(detectSyntax("```ts\ninterface Entry { id: string }\n```\n")).toBe("markdown");
        expect(detectSyntax("Title\n=====\n\nDescription.")).toBe("markdown");
        expect(detectSyntax("| Name | Value |\n| --- | --- |\n| mode | auto |\n")).toBe("markdown");
    });

    it("does not promote quoted templates or examples to the outer language", () =>
    {
        expect(detectSyntax('const value = "<Example />";')).toBe("javascript");
        expect(detectSyntax('const value = "interface Example { name: string }";')).toBe("javascript");
        expect(detectSyntax('const value = "{#if visible}<span>Hello</span>{/if}";')).toBe("javascript");
        expect(detectSyntax('const view = <Example title="hello" />;')).toBe("jsx");
    });

    it("recognizes Svelte templates before their embedded languages", () =>
    {
        for (
            const text of [
                '<script lang="ts">import { onMount } from "svelte"; const name: string = "World";</script>\n<h1>{name}</h1>',
                "<script>let count = $state(0);</script>\n<button>{count}</button>",
                "{#if visible}\n<p>Hello</p>\n{:else}\n<p>Hidden</p>\n{/if}",
                "{#each items as item}<div>{item.name}</div>{/each}",
                '{#snippet greeting(name)}<p>{name}</p>{/snippet}\n{@render greeting("World")}',
                "<input bind:value={name} />",
                "<svelte:window onresize={resize} />",
                "<button on:click={increment}>Count</button>"
            ]
        )
        {
            expect(detectSyntax(text), text).toBe("svelte");
        }
        expect(detectSyntax("```svelte\n{#if visible}<p>Hello</p>{/if}\n```")).toBe("markdown");
        expect(detectSyntax(JSON.stringify({ source: "{#if visible}Hello{/if}" }))).toBe("json");
        expect(detectSyntax("<div>Hello</div>")).toBe("html");
        expect(detectSyntax('import { onMount } from "svelte";')).toBe("javascript");
    });

    it("recognizes JSON from a sample ending inside a container", () =>
    {
        expect(detectSyntax(JSON.stringify({ values: Array.from({ length: 2000 }, (_, index) => index) }))).toBe(
            "json"
        );
    });

    it("recognizes JavaScript-compatible modules without requiring variable declarations", async () =>
    {
        const config = await readFile(new URL("../../../../../eslint.config.ts", import.meta.url), "utf8");
        expect(detectSyntax(config)).toBe("javascript");
        for (
            const text of [
                'import parser from "parser";',
                'import { defineConfig } from "config";',
                'import * as config from "config";',
                'import config, { defineConfig } from "config";',
                'import config, * as helpers from "config";',
                'import "side-effects";',
                'export { config } from "config";',
                'export * from "config";',
                "export default { enabled: true };",
                "export default [ { enabled: true } ];"
            ]
        )
        {
            expect(detectSyntax(text), text).toBe("javascript");
        }
        expect(detectSyntax(`${config}\nconst enabled: boolean = true;`)).toBe("typescript");
        expect(detectSyntax('"import parser from \\"parser\\";"')).toBeNull();
        expect(detectSyntax('// import parser from "parser";')).toBeNull();
        expect(detectSyntax("import settings from the previous installation")).toBeNull();
    });

    it("recognizes TypeScript ambient declarations, type imports and annotated variables", () =>
    {
        for (
            const text of [
                'declare module "example"\n{\n    export const value: string;\n}',
                'declare module "*.svelte";',
                "export declare namespace Example { const value: string; }",
                "declare global { interface Window { example: string; } }",
                'import type { Component } from "svelte";',
                'import type Component from "svelte";',
                "export const value: string = 'example';",
                "declare const value: string;"
            ]
        )
        {
            expect(detectSyntax(text), text).toBe("typescript");
        }
        expect(detectSyntax('const value = "example";')).toBe("javascript");
        expect(detectSyntax('```ts\ndeclare module "example";\n```')).toBe("markdown");
    });

    it("avoids common prose false positives and unknown aliases", () =>
    {
        for (
            const text of [
                "",
                "Hello world",
                "https://example.com/path",
                "Meeting: tomorrow\nLocation: office",
                "#hashtag",
                "[1, nope]",
                "We should declare module ownership before release.",
                "import type information from the documentation",
                "The const value: string describes a variable."
            ]
        )
        {
            expect(detectSyntax(text)).toBeNull();
        }
    });
});

describe("syntax detection regression corpus", () =>
{
    describe.each(syntaxSamples)("$language / $name", ({ language, text }) =>
    {
        // JSONC supports embedded tokenization, but is not guessed from comments.
        if (language !== "jsonc")
        {
            it.each([
                ["original", text],
                ["CRLF", text.replaceAll("\n", "\r\n")],
                ["leading BOM and whitespace", `\uFEFF \t\n${text}`]
            ])("detects %s without a filename", (_name, source) =>
            {
                expect(detectSyntax(source)).toBe(language);
            });
        }

        it("keeps a JSON wrapper as JSON", () =>
        {
            expect(detectSyntax(JSON.stringify({ source: text }))).toBe("json");
        });

        it("keeps a Markdown fence as Markdown", () =>
        {
            expect(detectSyntax(`\`\`\`${language}\n${text}\n\`\`\``)).toBe("markdown");
        });

        it("does not detect code inside a JavaScript string as the outer language", () =>
        {
            expect(detectSyntax(`const example = ${JSON.stringify(text)};`)).toBe("javascript");
        });

        it("does not detect code inside a template literal as the outer language", () =>
        {
            const escaped = text.replaceAll("\\", "\\\\").replaceAll("`", "\\`").replaceAll("${", "\\${");
            expect(detectSyntax(`const example = \`\n${escaped}\n\`;`)).toBe("javascript");
        });

        it("does not detect code inside a block comment as the outer language", () =>
        {
            const comment = text.replaceAll("*/", "* /");
            expect(detectSyntax(`const example = 1;\n/*\n${comment}\n*/`)).toBe("javascript");
        });
    });

    it.each([
        ["eslint.config.ts", "javascript"],
        ["justfile", "just"],
        ["vitest.config.ts", "javascript"],
        ["package.json", "json"],
        ["THIRD_PARTY_NOTICES.md", "markdown"],
        ["apps/desktop/frontend/src/styles/global.css", "css"],
        ["Cargo.toml", "toml"],
        ["apps/desktop/frontend/src/generated/TextRead.ts", "typescript"],
        ["apps/desktop/frontend/tests/syntax/fixtures/modules.txt", "typescript"],
        ["apps/desktop/frontend/verification/syntax/fixtures.ts", "typescript"],
        ["apps/desktop/frontend/tests/syntax/fixtures/objective-c.m", "objective-c"],
        ["apps/desktop/frontend/src/components/extensions/TextPreview.svelte", "svelte"],
        ["apps/extensions/built-in/clipboard/src/ClipboardConfig.rs", "rust"],
        ["engine/runtime/src/ExtensionRuntimeInvocation.rs", "rust"]
    ])("detects copied repository file %s as %s", async (path, language) =>
    {
        const text = await readFile(new URL(`../../../../../${path}`, import.meta.url), "utf8");
        expect(detectSyntax(text)).toBe(language);
    });

    it.each([
        "Call me tomorrow at noon.",
        "请在明天下午三点开会，讨论 TypeScript 和 Rust。",
        "https://example.com/config.ts?language=typescript",
        "C:\\Users\\Example\\config.ts",
        "/Users/example/project/src/main.rs",
        "2026-10-13T12:00:00Z INFO worker started",
        "name = value",
        "enabled: true",
        "42",
        "// import parser from 'parser';",
        "/* const example: string = 'hello'; */"
    ])("abstains on ambiguous or non-code text: %s", text =>
    {
        expect(detectSyntax(text)).toBeNull();
    });

    it("does not inspect language declarations beyond the detection sample", () =>
    {
        const text = `${" ".repeat(DETECTION_LIMIT)}interface Example { name: string; }`;
        expect(detectSyntax(text)).toBeNull();
    });

    it("classifies the prefix without validating trailing JSON content", () =>
    {
        const text = `${JSON.stringify({ value: "x".repeat(DETECTION_LIMIT) })} invalid`;
        expect(detectSyntax(text)).toBe("json");
        expect(detectSyntax(`\uFEFF${text}`)).toBe("json");
        expect(detectSyntax(text.slice(0, DETECTION_LIMIT))).toBe("json");
    });
});

it("keeps detection independent of document length and suffix", () =>
{
    for (
        const prefix of [
            `{"text":"${"x".repeat(DETECTION_LIMIT)}`,
            `[{"text":"${"x".repeat(DETECTION_LIMIT)}`,
            `[${"1,".repeat(DETECTION_LIMIT)}`,
            `const value = "${"x".repeat(DETECTION_LIMIT)}`
        ]
    )
    {
        const sample = prefix.slice(0, DETECTION_LIMIT);
        expect(detectSyntax(sample + "invalid".repeat(100_000))).toBe(detectSyntax(sample));
    }
});

it("recognizes Just settings without promoting comments to Markdown", async () =>
{
    const text = await readFile(new URL("../../../../../justfile", import.meta.url), "utf8");
    expect(detectSyntax(text)).toBe("just");
    expect(detectSyntax('set shell := ["bash", "-cu"]\n\n# Build application\nbuild:\n    cargo build')).toBe("just");
    expect(detectSyntax(`\`\`\`just\n${text}\n\`\`\``)).toBe("markdown");
    expect(detectSyntax(JSON.stringify({ source: text }))).toBe("json");
});

it.each([
    ["bare recipe", "build:\n    cargo build", null],
    ["Makefile assignment", "CC := clang\nall: main.c\n\t$(CC) main.c", null],
    ["Makefile interpolation", "NAME := app\nbuild:\n\techo {{NAME}}", null],
    ["TOML section", '[private]\nenabled = true\nname = "build"', "toml"],
    ["INI section", "[unix]\ncommand = make\nmode = fast", "ini"],
    ["YAML block scalar", 'script: |\n  set shell := ["bash", "-cu"]\nnext: true', "yaml"],
    ["Shell shebang", '#!/bin/bash\n# Example syntax\nset shell := ["bash", "-cu"]', "shell"],
    ["Python string", 'def example():\n    return \'set shell := ["bash", "-cu"]\'', "python"],
    ["JavaScript template", 'const example = `set shell := ["bash", "-cu"]`;', "javascript"],
    ["Markdown prose", '# Build guide\n\nUse **this setting**:\nset shell := ["bash", "-cu"]', "markdown"],
    ["unknown setting", 'set arbitrary-option := "value"', null],
    ["bare attribute", "[private]", null],
    ["unrelated shebang", "#!/usr/bin/env bash # just an example\necho hello", "shell"]
])("does not misclassify %s as Just", (_name, source, expected) =>
{
    expect(detectSyntax(source)).toBe(expected);
});

it.each([
    "# See [build docs](https://example.com)\nbuild:\n    cargo build",
    "# Build application\nbuild:\n    cargo build",
    "# Build application\n\nbuild:\n    cargo build",
    "# Build application\n\necho hello",
    "# Build\n\n## Run\n\nbuild:\n    cargo build",
    "# Setup\n\n## Notes\n\necho hello",
    "# Load settings\n\nNAME := app\nall:\n\techo $(NAME)",
    "# Configuration\n\n[private]",
    "# Just imports\nimport 'common.just'\nmod tasks",
    '# Example\nvalue = "**quoted emphasis**"',
    '# Example\nvalue = "[guide](https://example.com)"',
    "# Example\n// See [guide](https://example.com)"
])("does not infer Markdown from hash comments or an ambiguous heading: %s", text =>
{
    expect(detectSyntax(text)).not.toBe("markdown");
});

it.each([
    "# Heading\n\nSome **bold** text.",
    "# Heading\n\nPlain text with no inline formatting.",
    "# Heading\n\nText before code with `inline` values.",
    `# Document\n\n${"Text before code with `inline` values.\n".repeat(440)}`,
    "# 标题\n\n这是一段没有链接的正文。",
    "# Notices\n\n## Package\n\nLicense text follows",
    "# Guide\n\nThis paragraph uses a colon: words.",
    "# Heading\n\nSee [the guide](https://example.com).",
    "# Heading\n\nSee [the guide](./guide.md).",
    "Title\n=====\n\nDescription.",
    "# Tasks\n\n- [x] Done",
    "| Name | Value |\n| --- | --- |\n| mode | auto |"
])("keeps independent Markdown evidence: %s", text =>
{
    expect(detectSyntax(text)).toBe("markdown");
});

it.each([
    ":root { --accent: #2563eb; }",
    "body { color: red; }",
    "* { box-sizing: border-box; }",
    "button:hover { color: red; }",
    '@font-face { font-family: "Example"; src: url(example.woff2); }',
    "@media (width < 800px) {\n  :root { --size: 12px; }\n}",
    '[data-state="open"] { opacity: 1; }'
])("recognizes CSS selectors and declarations: %s", text =>
{
    expect(detectSyntax(text)).toBe("css");
});
