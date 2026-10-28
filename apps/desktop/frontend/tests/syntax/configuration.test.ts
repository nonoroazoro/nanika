import { describe, expect, it } from "vitest";

import { detectConfiguration } from "../../src/syntax/configuration";
import { detectSyntax } from "../../src/syntax/detect";
import { DETECTION_LIMIT } from "../../src/syntax/limits";

describe("configuration recognition", () =>
{
    it.each([
        ["INI apostrophes", "[message]\ntext = don't stop, it's fine\nstatus = ready", "ini"],
        ["INI paths", "[paths]\nshare = //server/share\nwindows = \\\\server\\share", "ini"],
        ["quoted comment markers", '[settings]\ntext = "Keep # [brackets] and ; punctuation"\nenabled = true', "toml"],
        ["literal backslashes", "[settings]\npath = 'C:\\Users\\example'\nenabled = true", "toml"],
        ["multiline array comments", '[settings]\nvalues = [\n  "a", # [ { ignored\n  "b"\n]\nenabled = true', "toml"],
        ["literal multiline value", "[settings]\ntext = '''\nDon't stop.\nIt's fine.\n'''\nenabled = true", "toml"],
        ["trailing literal quotes", '[settings]\ntext = """a quoted word: "hello""""\nenabled = true', "toml"],
        ["mapping sequences", "items:\n  - name: one\n    tags: [alpha, beta]\n  - name: two", "yaml"],
        ["literal YAML apostrophe", "server:\n  message: don't stop, it's fine\n  name: 'It''s fine'", "yaml"],
        ["indentless YAML sequence", "items:\n- alpha\n- beta\nenabled: true", "yaml"],
        ["explicit YAML", "---\nname: example\n", "yaml"],
        ["YAML alias cycle", "items: &items [*items]\nenabled: true", "yaml"],
        ["HTTP headers", "Content-Length: 123\nContent-Type: application/json", null],
        ["numeric status labels", "code: 404\nmessage: Not Found", null],
        ["mismatched TOML collection", "[settings]\nvalues = [}\nenabled = true", null],
        ["newline in a basic string", '[settings]\ntext = "not\na multiline string"\nenabled = true', null],
        ["mismatched YAML collection", "items: [}\nenabled: true", null],
        ["invalid YAML indentation", "server:\n  host: localhost\n port: 8080", null],
        ["configuration followed by prose", "enabled: true\nretries: 3\nPlease restart the app.", null]
    ])("handles %s without interpreting its payload", (_name, text, expected) =>
    {
        expect(detectConfiguration(text, false)).toBe(expected);
    });

    it.each(
        [
            ["toml", "[settings]\nenabled = true\ncount = 2\n", [
                "another = 42\n",
                'text = "Keep \\"quotes\\" # brackets []"\n',
                'values = [\n  "first",\n  "second"\n]\n',
                'text = """\nMultiline content.\n"""\n'
            ]],
            ["yaml", "enabled: true\ncount: 2\n", [
                "another: 42\n",
                'text: "Keep \\"quotes\\" # brackets []"\n',
                "values: [\n  first,\n  second\n]\n",
                "values:\n  - first\n  - second\n",
                "text: |\n  Multiline content.\n"
            ]]
        ] as const
    )("keeps %s evidence at every sampling boundary", (language, prefix, fragments) =>
    {
        for (const fragment of fragments)
        {
            for (let cut = 1; cut <= fragment.length; cut++)
            {
                const padding = `${"#".padEnd(DETECTION_LIMIT - prefix.length - cut - 1, "x")}\n`;
                const text = prefix + padding + fragment;
                expect(detectSyntax(text), `${language}: ${JSON.stringify(fragment.slice(0, cut))}`).toBe(language);
                expect(detectSyntax(text.slice(0, DETECTION_LIMIT))).toBe(language);
            }
        }
    });

    it.each(["[settings]\nenabled = true\ncount = 2\n", "enabled: true\ncount: 2\n"])(
        "does not hide complete invalid lines before a truncated tail",
        prefix =>
        {
            const text = `${prefix}This is prose.\n#${"x".repeat(DETECTION_LIMIT)}\nname: value`;
            expect(detectSyntax(text)).toBeNull();
        }
    );
});
