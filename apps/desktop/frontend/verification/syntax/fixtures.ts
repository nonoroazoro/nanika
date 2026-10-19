import original from "./fixtures.json";
import justfile from "../../../../../justfile?raw";
import svelteComponent from "../../src/components/extensions/TextPreview.svelte?raw";

import type { SyntaxFixture } from "./SyntaxFixture";

/**
 * Original corpus bytes remain unchanged so baseline comparisons remain reproducible.
 */
export const fixtures: SyntaxFixture[] = [
    ...original,
    { id: "justfile", language: "just", text: justfile },
    {
        id: "just-4096",
        language: "just",
        text: `set shell := ["bash", "-cu"]\n${
            Array.from({ length: 48 }, (_, index) =>
                `# Build target ${index}\nbuild-${index} target="release":\n    cargo build --profile {{target}}\n`)
                .join("")
        }`
    },
    { id: "svelte-component", language: "svelte", text: svelteComponent },
    {
        id: "svelte-500",
        language: "svelte",
        text: `<script lang="ts">\nlet count: number = $state(0);\n</script>\n${
            "{#if count > 0}\n<button onclick={() => count++}>{count}</button>\n{:else}\n<p>Empty</p>\n{/if}\n".repeat(
                100
            )
        }<style>\nbutton { color: red; }\n</style>\n`
    },
    { id: "dense-json", language: "json", text: JSON.stringify(Array.from({ length: 1500 }, () => 1)) },
    {
        id: "dense-markdown",
        language: "markdown",
        text: `# Dense Markdown\n${"`x` *y* **z** [a](b) ".repeat(8).concat("\n").repeat(300)}`
    },
    {
        id: "dense-unicode",
        language: "markdown",
        text: `# Unicode Markdown\n${"`你😀` *é* **אבג** [文](路) ".repeat(6).concat("\n").repeat(200)}`
    },
    {
        id: "unicode-markdown",
        language: "markdown",
        text: "# 😀 é אבג\r\n\r\n> ```ts\r\n> const value = '😀';\r\n> ```\r\n\r\n- [x] Done\r\n"
    },
    {
        id: "html",
        language: "html",
        text: '<!DOCTYPE html>\n<main class="page"><h1>Hello</h1><p>Text</p></main>\n'.repeat(40)
    },
    {
        id: "jsx",
        language: "jsx",
        text: 'export function View() { return <main title="Hello">{value}</main>; }\n'.repeat(40)
    },
    {
        id: "tsx",
        language: "tsx",
        text:
            "interface Props { value: string; }\nexport function View({value}: Props) { return <main>{value}</main>; }\n"
                .repeat(30)
    },
    { id: "jsonc", language: null, text: '{\n  // Configuration\n  "enabled": true,\n  "values": [1, 2, 3],\n}\n' },
    {
        id: "unclosed-markdown",
        language: "markdown",
        text: "# Unclosed\n\n- Quote:\n  > ```python\n  > def run():\n  >     return 'value'\n"
    },
    {
        id: "boundary-fence",
        language: "markdown",
        text: `# Document\n\n${
            "Text before code with `inline` values.\n".repeat(440)
        }\n\`\`\`ts\n/* start\ncontinued */\nconst finalValue = 'tail';\n\`\`\`\n`
    },
    ...[4095, 4096, 4097].map(length => ({
        id: `line-${length}`,
        language: "javascript",
        text: `const value = '${"x".repeat(length - 17)}';`
    })),
    { id: "plain-url", language: null, text: "https://example.com/path?query=1#section" },
    { id: "plain-log", language: null, text: "2026-10-09T00:00:00Z INFO request completed (200)\n".repeat(300) }
];
