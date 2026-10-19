import { DETECTION_LIMIT } from "./limits";

import type { SyntaxLanguage } from "./SyntaxLanguage";

// Ambient files may have no interface or type alias. Declaration punctuation
// distinguishes these bounded-sample signals from prose about TypeScript.
const typescriptDeclarations = [
    /\b(?:interface +\w+\s*(?:extends[^\n{]+)?\{|type +\w+(?:<[^>]+>)? *=)/,
    /^[ \t]*(?:export[ \t]+)?declare[ \t]+(?:(?:module|namespace)[ \t]+(?:[$\w.]+|"[^"\r\n]+"|'[^'\r\n]+')|global)\s*[{;]/m,
    /^[ \t]*import[ \t]+type[ \t]+(?:[$\w]+|\{[^}]+\}|\*[ \t]+as[ \t]+[$\w]+)\s+from\s+["']/m,
    /^[ \t]*(?:export[ \t]+)?(?:declare[ \t]+)?(?:const|let|var)[ \t]+[$A-Z_a-z][$\w]*\s*:[^=;\r\n]+[=;]/m
];

// JS-compatible modules remain JavaScript when the copied text has no TS-only syntax.
const javascriptModules = [
    /^[ \t]*import[ \t]+(?:[$\w]+|\{[^}]*\}|\*[ \t]+as[ \t]+[$\w]+)(?:\s*,\s*(?:\{[^}]*\}|\*[ \t]+as[ \t]+[$\w]+))?\s+from\s*["']/m,
    /^[ \t]*import[ \t]*["'][^"'\r\n]*["'][ \t]*;?[ \t]*(?:\r?$)/m,
    /^[ \t]*export[ \t]+default[ \t]+(?:[[{]|(?:async[ \t]+)?function\b|class\b)/m,
    /^[ \t]*export[ \t]+(?:\{[^}]*\}|\*(?:[ \t]+as[ \t]+[$\w]+)?)[ \t]+from\s*["']/m
];

const svelteTemplates = [
    /\{(?:#(?:if|each|await|key|snippet)\s+[^}]+|@(?:html|render|const|debug)\s+[^}]+|:(?:else|then|catch)\b[^}]*|\/(?:if|each|await|key|snippet))\}/,
    /<svelte:[a-z]+\b/,
    /<[A-Za-z][\w.-]*\b[^>]*\s(?:bind|on|use|transition|in|out|animate|class|style):[\w-]+/
];
const svelteScript =
    /(?:from\s*["']svelte(?:\/[^"']*)?["']|\$(?:state|derived|effect|props|bindable|inspect|host)\s*(?:\.[a-z]+\s*)?\()/;

// Strong document prefixes take priority over code embedded in strings. The
// complete grammars still own tokenization; these signals only choose the outer language.
const codePrefixes: Array<[SyntaxLanguage, RegExp]> = [
    [
        "objective-c",
        /^(?:#[ \t]*import[ \t]+[<"]|@(?:interface|implementation|protocol|class)[ \t]+\w+|@autoreleasepool\s*\{)/
    ],
    ["php", /^<\?php\b/],
    ["xml", /^(?:<\?xml\b|<!DOCTYPE\s+(?!html\b)|<[\w:.-]+\b[^>]*\sxmlns(?::\w+)?=)/i],
    ["go", /^package[ \t]+\w+[ \t]*(?:\r?\n|$)/],
    ["csharp", /^(?:(?:global[ \t]+)?using[ \t]+(?:static[ \t]+)?[\w.]+[ \t]*;|namespace[ \t]+[\w.]+\s*[;{])/],
    ["java", /^(?:package[ \t]+[\w.]+[ \t]*;|import[ \t]+(?:static[ \t]+)?[\w.]+\*?[ \t]*;)/],
    [
        "swift",
        /^(?:import[ \t]+(?:Foundation|SwiftUI|UIKit|AppKit|Swift)\b|(?:public[ \t]+)?func[ \t]+\w+\s*\([^)]*\w+\s*:\s*\w+)/
    ],
    [
        "powershell",
        /^(?:param\s*\(|function[ \t]+\w+-\w+|(?:Get|Set|Write|New|Remove|Import|Test|Invoke|Start|Stop)-\w+\b)/i
    ],
    ["scss", /^(?:\$[\w-]+\s*:|@(?:mixin|include|use|forward)\b)/],
    ["less", /^@[\w-]+\s*:/],
    ["lua", /^(?:local[ \t]+(?:function[ \t]+\w+|\w+[ \t]*=)|--[^\n]*\n\s*local\b)/]
];

/**
 * Guess the outer document conservatively. This does not parse Markdown or try
 * every grammar. TextMate remains responsible for all syntax boundaries.
 *
 * @param text Source prefix. At most DETECTION_LIMIT UTF-16 code units are inspected.
 */
export function detectSyntax(text: string): SyntaxLanguage | null
{
    const sample = text.slice(0, DETECTION_LIMIT).trimStart();
    if (!sample)
    {
        return null;
    }
    const code = _codeSignals(sample);
    if (/^(?: {0,3}> ?)* {0,3}(?:`{3,}|~{3,})[^\n]*$/m.test(code))
    {
        return "markdown";
    }
    if (sample.startsWith("{") || sample.startsWith("["))
    {
        try
        {
            // Classification never validates or reads the rest of the document.
            JSON.parse(sample);
            return "json";
        }
        catch
        {
            // A full sample may end inside a JSON value. Require a structured JSON
            // prefix, not just an opening bracket; this is recognition, not validation.
            if (text.length >= DETECTION_LIMIT && _jsonPrefix(sample))
            {
                return "json";
            }
        }
    }
    // Match the actual interpreter, not a language name later in shebang arguments
    // or a trailing comment. A known executable outranks comment-like headings.
    const interpreter =
        /^#![ \t]*\/(?:[^\s/]+\/)*(?:env[ \t]+(?:-S[ \t]+)?)?(just|bash|zsh|sh|python[\d.]*|ruby)(?=[ \t\r\n]|$)/.exec(
            sample
        )?.[1];
    if (interpreter)
    {
        if (interpreter === "just" || interpreter === "ruby")
        {
            return interpreter;
        }
        return interpreter.startsWith("python") ? "python" : "shell";
    }
    const head = code.trimStart();
    for (const [language, prefix] of codePrefixes)
    {
        if (prefix.test(head))
        {
            return language;
        }
    }
    if (
        /^(?:#[ \t]*(?:include|pragma)\b|(?:static[ \t]+)?(?:int|void|char|float|double|struct)[ \t]+\w+[ \t]*[(*{])/
            .test(head)
    )
    {
        return /\bstd::|\btemplate\s*<|\busing[ \t]+namespace\b|#[ \t]*include[ \t]*<[a-z_]+>/.test(head)
            ? "cpp"
            : "c";
    }
    if (/^(?:template\s*<|using[ \t]+namespace[ \t]+\w+\s*;|std::)/.test(head))
    {
        return "cpp";
    }
    const configuration = sample.replace(/^(?:\s|[#;][^\n]*(?:\n|$))+/, "");
    if (/^(?:ARG[ \t]+[^\n]+\n\s*)*FROM[ \t]+[\w./:@-]+(?:[ \t]+AS[ \t]+\w+)?[ \t]*(?:\r?\n|$)/i.test(configuration))
    {
        return "dockerfile";
    }
    const assignments = configuration.match(/^[ \t]*[\w."'-]+[ \t]*=[ \t]*[^\n]+/gm) ?? [];
    const tomlValues = assignments.filter(line => /^[^=]+=[ \t]*(?:["'[{]|(?:true|false)\b|[+-]?\d)/.test(line));
    if (/^\[\[?[^\]\n]+\]\]?[ \t]*(?:\r?\n|$)/.test(configuration) && assignments.length > 0)
    {
        return tomlValues.length === assignments.length ? "toml" : "ini";
    }
    if (tomlValues.length >= 2 && assignments.length === tomlValues.length && /^[\w."'-]+[ \t]+=/.test(configuration))
    {
        return "toml";
    }
    if (/^(?:require[ \t]+["']|def[ \t]+\w+[^\n:]*\n)[\s\S]*?^[ \t]*end\b/m.test(code))
    {
        return "ruby";
    }
    if (_justSyntax(sample))
    {
        return "just";
    }
    // Hash-prefixed lines are comments in many languages. A heading alone is
    // insufficient: require independent Markdown structure outside quoted code.
    const heading = /^#{1,6}[ \t]+\S[^\n]*$/m.test(code);
    const body = code.replace(/^#{1,6}[ \t]+[^\n]*$/gm, "");
    const inlineMarkup = /\*\*[^*\n]+\*\*|__[^_\n]+__|\[[^\]\n]+\]\([^)\n]+\)/.test(body);
    if (
        (heading && inlineMarkup)
        || /\S[^\n]*\n(?:={3,}|-{3,})\s*(?:\n|$)/.test(code)
        || /^\|?.+\|.+\n\|? *:?-{3,}:? *\|/m.test(code)
        || /^[-*+] +\[[ xX]\] +\S/m.test(code)
        || (/^[-*+] +\S/m.test(code) && /(?:\*\*[^*\n]+\*\*|\[[^\]\n]+\]\([^)\n]+\))/.test(code))
    )
    {
        return "markdown";
    }
    // Svelte-specific template syntax wins over embedded TS/JS and HTML.
    // A script tag alone is shared by other formats and is not sufficient.
    if (
        svelteTemplates.some(pattern => pattern.test(code))
        || (/<script\b[^>]*>/.test(code) && svelteScript.test(sample))
    )
    {
        return "svelte";
    }
    if (/^(?:async +)?def +\w+\([^\n]*\)\s*(?:->[^\n]+)?\s*:/m.test(code))
    {
        return "python";
    }
    if (
        /^[ \t]*(?:pub(?:\([^)]*\))?[ \t]+)?(?:(?:async|const|unsafe)[ \t]+)*fn[ \t]+\w+\s*[<(]/m.test(code)
        || /^[ \t]*(?:pub(?:\([^)]*\))?[ \t]+)?use[ \t]+\w+::/m.test(code)
        || /^[ \t]*pub(?:\([^)]*\))?[ \t]+(?:struct|enum|trait)[ \t]+\w+\s*[<{(;]/m.test(code)
        || /^[ \t]*(?:unsafe[ \t]+)?(?:impl(?:[ \t]+|<)|trait[ \t]+)\w[^;{}]*\{/m.test(code)
    )
    {
        return "rust";
    }
    if (/^(?:<!doctype html\b|<html\b)/i.test(head))
    {
        return "html";
    }
    const jsx = /<>[\s\S]*?<\/>|<[A-Za-z][\w.:-]*\b[^<>]*\/>/.test(code)
        || /<([A-Za-z][\w.:-]*)\b[^<>]*>[\s\S]*?<\/\1\s*>/i.test(code);
    if (typescriptDeclarations.some(pattern => pattern.test(code)))
    {
        return jsx ? "tsx" : "typescript";
    }
    if (/\b(?:SELECT\s+[\s\S]+?\s+FROM|INSERT\s+INTO|CREATE\s+TABLE|UPDATE\s+\w+\s+SET)\b/i.test(code))
    {
        return "sql";
    }
    if (
        javascriptModules.some(pattern => pattern.test(code))
        || /\b(?:const|let|var) +[$\w]+\s*=|\b(?:export +)?(?:async +)?function +[$\w]*\s*\(/.test(code)
    )
    {
        return jsx ? "jsx" : "javascript";
    }
    if (/^<[a-z][\w-]*\b/i.test(head) && jsx)
    {
        return "html";
    }
    if (
        /^[ \t]*[\w.#:*@[][\w\s.#:*[\]="'|~^$>+(),-]*\{\s*(?:--)?[a-zA-Z][\w-]*\s*:[^{};]+[;}]/m.test(code)
    )
    {
        return "css";
    }
    if (
        /^(?:---\s*\n)?[\w.-]+:\s*[^\n]*\n[ \t]+[\w.-]+:\s/m.test(code)
        || /^-[ \t]+[\w.-]+:[ \t]+\S/m.test(configuration)
        || ((configuration.match(/^[a-z][\w.-]*:[ \t]+\S/gm)?.length ?? 0) >= 2
            && /^[a-z][\w.-]*:[ \t]+(?:true|false|null|[+-]?\d|\[|\{|[|>])/m.test(configuration))
    )
    {
        return "yaml";
    }
    // A document title followed by prose is useful Markdown evidence on its own.
    // Run this after code signals so hash comments cannot outrank actual syntax.
    if (/^#{1,6}[ \t]+[^\n]+\r?\n[ \t]*\r?\n/.test(sample))
    {
        // A sampled final line may end inside inline markup, so it is not evidence against earlier prose.
        const proseSample = text.length >= DETECTION_LIMIT ? sample.slice(0, sample.lastIndexOf("\n")) : sample;
        const paragraphs = proseSample.split(/\r?\n[ \t]*\r?\n/).slice(1);
        const prose = paragraphs.some(paragraph =>
        {
            const plain = paragraph.replace(/`[^`\r\n]+`/g, "code");
            return /^[\p{L}\p{N}][\p{L}\p{N}\s,'’"“”():/.!?。！？、，；：-]*$/u.test(plain)
                && !/:[ \t]*(?:\r?\n|$)/.test(paragraph)
                && ((plain.match(/[\p{L}\p{N}]+/gu)?.length ?? 0) >= 3 || /\p{Script=Han}{8}/u.test(paragraph));
        });
        if (prose)
        {
            return "markdown";
        }
    }
    return null;
}

/**
 * Hide quoted bodies and comments when checking JS/TS and template signals.
 * Preserve quotes, newlines, URL schemes and unquoted Markdown fences so
 * outer-document signals survive.
 *
 * @param sample Bounded document sample
 */
function _codeSignals(sample: string): string
{
    return sample.replace(
        /^(?: {0,3}> ?)* {0,3}(?:`{3,}|~{3,})[^\n]*$|<!--[\s\S]*?(?:-->|(?![\s\S]))|\/\*[\s\S]*?(?:\*\/|(?![\s\S]))|:\/\/|\/\/[^\r\n]*|"""[\s\S]*?(?:"""|(?![\s\S]))|'''[\s\S]*?(?:'''|(?![\s\S]))|"(?:\\[\s\S]?|[^"\\\r\n])*"?|'(?:\\[\s\S]?|[^'\\\r\n])*'?|`(?:\\[\s\S]?|[^`\\])*(?:`|(?![\s\S]))/gm,
        value =>
        {
            // Matching fences before quoted tokens preserves real Markdown, while a string
            // opened earlier consumes any fences inside it as part of that string.
            if (/^(?: {0,3}> ?)* {0,3}(?:`{3,}|~{3,})[^\n]*$/.test(value))
            {
                return value;
            }
            if (value === "://")
            {
                return value;
            }
            const blank = value.replace(/[^\r\n]/g, " ");
            if (value.startsWith("/") || value.startsWith("<"))
            {
                return blank;
            }
            // Ordinary quotes stop at unescaped newlines; multiline tokens consume the
            // sample through EOF when truncated, so their contents cannot become signals.
            const closing = value.length > 1 && value.at(-1) === value[0] ? value.at(-1) : " ";
            return value.length === 1 ? value : value[0] + blank.slice(1, -1) + closing;
        }
    );
}

/**
 * Recognize JSON containers when sampling cuts off the closing structure.
 * Trailing content outside the sample cannot affect language classification.
 */
function _jsonPrefix(sample: string): boolean
{
    const member =
        /^\{\s*"(?:[^"\\\r\n]|\\(?:["\\/bfnrt]|u[\da-fA-F]{4}))*"\s*:\s*(?:["{[]|-?\d|true\b|false\b|null\b)/;
    if (member.test(sample))
    {
        return true;
    }
    if (!sample.startsWith("["))
    {
        return false;
    }
    const first = sample.slice(1).trimStart();
    return member.test(first)
        || /^(?:"(?:[^"\\\r\n]|\\(?:["\\/bfnrt]|u[\da-fA-F]{4}))*"|-?\d+(?:\.\d+)?(?:[eE][+-]?\d+)?|true|false|null)\s*[,\]]/
            .test(first);
}

/**
 * Require a Just-specific preamble or an attribute plus a recipe. Bare recipe
 * headers and := assignments also occur in Makefiles and are deliberately ambiguous.
 */
function _justSyntax(sample: string): boolean
{
    if (sample.startsWith("#!"))
    {
        return false;
    }
    // Only the document preamble can select Just. Do not search inside another
    // language's string, Markdown example, YAML block scalar or recipe body.
    const preamble = sample.replace(/^(?:[ \t]*#[^\r\n]*(?:\r?\n|$)|\s)+/, "");
    if (
        /^set[ \t]+(?:shell|windows-shell|script-interpreter)[ \t]*:=[ \t]*\[\s*["']/.test(preamble)
        || /^set[ \t]+(?:minimum-version|working-directory|tempdir|dotenv-(?:filename|path))[ \t]*:=[ \t]*["']/.test(
            preamble
        )
        || /^set[ \t]+(?:allow-duplicate-(?:recipes|variables)|dotenv-(?:load|override|required)|export|fallback|ignore-comments|positional-arguments|quiet|unstable|windows-powershell)[ \t]*:=[ \t]*(?:true|false)\b/
            .test(preamble)
    )
    {
        return true;
    }
    // An attribute alone is indistinguishable from a TOML section. Require the
    // immediately following recipe and its indented command body as independent evidence.
    return /^(?:\[(?:private|unix|windows|linux|macos|no-cd|no-exit-message|no-quiet|no-deps|positional-arguments)\][ \t]*\r?\n)+[ \t]*@?[\w-]+(?:[ \t]+[$+*]?[\w-]+(?:=(?:"[^"\r\n]*"|'[^'\r\n]*'|[^\s:]+))?)*:[^\r\n]*\r?\n[ \t]+\S/
        .test(preamble);
}
