import type { SyntaxLanguage } from "../../src/syntax/SyntaxLanguage";

export const languageSamples: Record<SyntaxLanguage, Record<string, string>> = {
    c: {
        basic: '#include <stdio.h>\nint main(void) { printf("hello"); return 0; }',
        function: "static int sum(int a, int b) { return a + b; }"
    },
    cpp: {
        basic: "#include <vector>\nstd::vector<int> values = {1, 2};",
        template: "template <typename T>\nT twice(T value) { return value + value; }"
    },
    csharp: {
        basic: 'using System;\nclass Example { static void Main() { Console.WriteLine("hello"); } }',
        namespace: "namespace Example;\npublic record Item(string Name);"
    },
    css: {
        basic: ".example { color: red; }",
        id: "#panel { display: grid; gap: 12px; }"
    },
    dockerfile: {
        basic: 'FROM alpine:3\nRUN echo "hello"\nCOPY . /app',
        stages: "ARG VERSION=22\nFROM node:22 AS build\nWORKDIR /app\nRUN npm ci"
    },
    go: {
        basic: 'package main\nimport "fmt"\nfunc main() { fmt.Println("hello") }',
        function: "package sample\n\nfunc Add(a, b int) int { return a + b }"
    },
    html: {
        uppercase: "<DIV>Hello</DIV>",
        mixedCase: "<DiV>Hello</dIv>",
        comments: "<!-- preview -->\n<!-- another comment -->\n<div>Hello</div>",
        basic: '<div class="example">hello</div>',
        document: "<!DOCTYPE html>\n<html><script>const value = 1;</script><main>Hello</main></html>",
        fragment: "<main><strong>Hello</strong></main>"
    },
    ini: {
        basic: "[server]\nhost = localhost\nport = 8080",
        comments: "; server configuration\n[network]\nhost=example.com\nport=443"
    },
    java: {
        basic:
            'package example;\npublic class Main { public static void main(String[] args) { System.out.println("hello"); } }',
        import: "import java.util.List;\nclass Example { List<String> names; }"
    },
    javascript: {
        longTemplate: `const docs = \`\n~~~rust\n${"example\n".repeat(600)}\n~~~\n\`;`,
        longComment: `const docs = 1;\n/*\n~~~rust\n${"example\n".repeat(600)}\n~~~\n*/`,
        basic: 'const message = "hello";',
        module: 'import { defineConfig } from "tool";\nexport default defineConfig({ enabled: true });',
        template: "const documentation = `\n# Heading\n\nDescription\n`;",
        fence: "const documentation = `\n~~~rust\nfn main() {}\n~~~\n`;"
    },
    json: {
        basic: '{"message":"hello", "enabled":true}',
        array: '[{"name":"hello","enabled":true},null,42]'
    },
    jsonc: {
        basic: '{\n// comment\n"message": "hello",\n}',
        trailingComma: '{\n/* settings */\n"enabled": true,\n"values": [1, 2,],\n}'
    },
    jsx: {
        basic: 'const view = <Example title="hello" />;',
        native: "export function View() { return <main>Hello</main>; }",
        div: "export function View() { return <div>Hello</div>; }",
        fragment: "const view = <><span>Hello</span><Widget /></>;",
        mixed: "const view = <section><UI.Button disabled={false} /></section>;"
    },
    just: {
        shellSetting: 'set shell := ["bash", "-cu"]\n\n# Build application\nbuild:\n    cargo build',
        versionSetting: 'set minimum-version := "1.56.0"\n\ndefault:\n    @just --list',
        commentedSettings: '# Settings\n\nset dotenv-load := true\nset working-directory := "."',
        attributedRecipe: '[private]\ndefault:\n    @just --list\n\nbuild +ARGS="":\n    cargo build {{ ARGS }}',
        platformAttributes: '[unix]\n[no-cd]\nbuild target="debug":\n    echo {{target}}',
        shebang: '#!/usr/bin/env -S just --justfile\n\nhello name:\n    echo "Hello {{name}}"'
    },
    less: {
        basic: "@color: red;\n.example { color: @color; }",
        mixin: "@spacing: 12px;\n.panel { padding: @spacing; &:hover { color: blue; } }"
    },
    lua: {
        basic: 'local message = "hello"\nprint(message)',
        function: 'local function greet(name)\n  return "hello " .. name\nend'
    },
    markdown: {
        contractions: "Here's an example:\n\n```ts\nconst value = 1;\n```\n\nThat's all.",
        quotation: 'The author wrote "hello.\n\n## Details\n\nText **continues** here."',
        basic: "# Heading\n\nSome **bold** text.",
        fence: '> ```typescript\n> const value: string = "hello";\n> ```',
        taskList: "- [x] Complete **review**\n- [ ] Add tests"
    },
    "objective-c": {
        basic: '#import <Foundation/Foundation.h>\n@interface Example : NSObject\n@end\nNSString *message = @"hello";',
        implementation: '@implementation Example\n- (NSString *)message { return @"hello"; }\n@end'
    },
    php: {
        basic: '<?php\n$message = "hello";\necho $message;',
        function: '<?php\nfunction greet(string $name): string { return "Hello " . $name; }'
    },
    powershell: {
        basic: 'Write-Host "hello"\nGet-ChildItem -Path .',
        parameters: "param([string]$Path)\nGet-Content -Path $Path | Write-Output"
    },
    python: {
        docstring: 'def greet():\n    """\n# Description\n\nA Markdown example.\n    """\n    return "hello"',
        basic: 'def greet(name):\n    return "hello"',
        async: 'async def fetch(client):\n    return await client.get("/items")'
    },
    ruby: {
        basic: 'def greet(name)\n  puts "hello"\nend',
        shebang: '#!/usr/bin/env ruby\nputs "hello"'
    },
    rust: {
        basic: 'fn main() { let message = "hello"; println!("{message}"); }',
        publicFunction: "pub fn add(a: i32, b: i32) -> i32 { a + b }",
        indentedMethod: "impl Entry {\n    pub fn new(value: String) -> Self { Self { value } }\n}",
        publicStruct: "#[derive(Debug, Clone)]\npub struct Entry {\n    pub(crate) value: String\n}",
        scopedEnum: "pub(crate) enum State { Ready, Pending }",
        tupleStruct: "pub struct Entry(String);",
        genericImpl: "impl<T> From<T> for Entry<T> {\n    fn from(value: T) -> Self { Self(value) }\n}",
        trait: "trait Render {\n    fn render(&self) -> String;\n}",
        nestedFunction: "mod inner {\n    pub(crate) async unsafe fn run() {}\n}"
    },
    scss: {
        basic: "$color: red;\n.example { color: $color; }",
        mixin: "@mixin panel($gap) { display: grid; gap: $gap; }\n.panel { @include panel(12px); }"
    },
    shell: {
        basic: '#!/bin/sh\necho "hello"',
        envShebang: '#!/usr/bin/env bash\nfor item in *.txt; do\n  printf "%s\\n" "$item"\ndone'
    },
    sql: {
        basic: "SELECT name FROM entries WHERE enabled = true;",
        update: "UPDATE entries SET enabled = false WHERE id = 1;"
    },
    svelte: {
        basic: "<script>let count = $state(0);</script>\n<button>{count}</button>",
        binding: "<input bind:value={name} />",
        script: '<script lang="ts">\nlet name: string = "Hello";\n</script>\n{#if name}<p>{name}</p>{/if}'
    },
    swift: {
        basic: 'import Foundation\nlet message: String = "hello"\nprint(message)',
        function: 'func greet(name: String) -> String { return "Hello " + name }'
    },
    toml: {
        basic: '# Settings\n[package]\nname = "example"\nversion = "1.0.0"',
        arrayTables: '[[items]]\nname = "first"\nenabled = true\n[[items]]\nname = "second"'
    },
    tsx: {
        basic: 'const view: JSX.Element = <Example title="hello" />;',
        native: "const view: JSX.Element = <main>Hello</main>;",
        fragment: "const view: JSX.Element = <><span>Hello</span></>;",
        mixed: "const view: JSX.Element = <section><UI.Button /></section>;"
    },
    typescript: {
        basic: 'const message: string = "hello";',
        typeImport: 'import type { Item } from "./Item";',
        generic: "const create = <T>(value: T) => value;\nconst count: number = 1;",
        assertion: "const count: number = <number>value;"
    },
    xml: {
        basic: '<?xml version="1.0"?>\n<config enabled="true"><name>example</name></config>',
        namespace: '<config xmlns="urn:example"><entry enabled="true" /></config>'
    },
    yaml: {
        basic: "server:\n  host: localhost\n  enabled: true",
        sequence: "- name: first\n  enabled: true\n- name: second\n  enabled: false"
    }
};

export const syntaxSamples = Object.entries(languageSamples).flatMap(([language, samples]) =>
{
    return Object.entries(samples).map(([name, text]) => ({ language, name, text }));
});
