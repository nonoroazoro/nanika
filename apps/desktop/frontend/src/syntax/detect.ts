import { detectConfiguration } from "./configuration";
import { DETECTION_LIMIT } from "./limits";

import type { SyntaxLanguage } from "./SyntaxLanguage";

// Ambient files may have no interface or type alias. Declaration punctuation
// distinguishes these bounded-sample signals from prose about TypeScript.
const typescriptDeclarations = [
    /^[ \t]*(?:export[ \t]+)?(?:interface +\w+\s*(?:extends[^\n{]+)?\{|type +\w+(?:<[^>]+>)? *=)/m,
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

const sqlIdentifier = String.raw`(?:[\w$]+|"[^"\r\n]+"|\x60[^\x60\r\n]+\x60|\[[^\]\r\n]+\])`;
const sqlQualifiedIdentifier = `${sqlIdentifier}(?:\\.${sqlIdentifier})*`;
const sqlStatement = new RegExp(
    String
        .raw`^(?:INSERT\s+INTO\s+${sqlQualifiedIdentifier}\s*(?:\([^)]*\)\s*)?(?:VALUES\s*\(|SELECT\s)|CREATE\s+TABLE\s+(?:IF\s+NOT\s+EXISTS\s+)?${sqlQualifiedIdentifier}\s*\(|UPDATE\s+${sqlQualifiedIdentifier}\s+SET\s+${sqlQualifiedIdentifier}\s*=)`,
    "i"
);
const sqlSelect = new RegExp(String.raw`^SELECT\s+([^;{}]+?)\s+FROM\s+${sqlQualifiedIdentifier}([\s\S]*)$`, "i");
const sqlAlias = new RegExp(String.raw`^(?:AS\s+)?${sqlIdentifier}(?=\s|;|$)\s*`, "i");

// Strong document prefixes take priority over code embedded in strings. The
// complete grammars still own tokenization; these signals only choose the outer language.
const codePrefixes: Array<[SyntaxLanguage, RegExp]> = [
    [
        "objective-c",
        /^(?:#[ \t]*import[ \t]+(?:<[^<>\s]+>|"[^"\r\n]+")|@(?:interface|implementation|protocol)[ \t]+\w+[ \t]*(?:[:<]|\r?\n)|@class[ \t]+\w+(?:[ \t]*,[ \t]*\w+)*[ \t]*;|@autoreleasepool\s*\{)/
    ],
    ["php", /^<\?php(?=\s|$)/],
    [
        "xml",
        /^(?:<\?xml[ \t]+[^?]*\?>|<!DOCTYPE\s+(?!html\b)[\w:.-]+\s*(?:\[[\s\S]*?\]|[^<>]*)>|<[\w:.-]+\b[^>]*\sxmlns(?::\w+)?=["'][^<>]*>)/i
    ],
    ["go", /^package[ \t]+\w+[ \t]*\r?\n\s*(?:import\s*["'(]|func\s+[\w(]|(?:type|var|const)\s+[\w(])/],
    [
        "csharp",
        /^(?:(?:global[ \t]+)?using[ \t]+(?:static[ \t]+)?[\w.]+[ \t]*;[ \t]*(?:\r?\n|$|(?=(?:using|namespace|class|struct|interface|enum|record|public|internal|static|abstract|sealed)\b))|namespace[ \t]+[\w.]+\s*[;{])/
    ],
    [
        "java",
        /^(?:package[ \t]+[\w.]+|import[ \t]+(?:static[ \t]+)?\w+(?:\.\w+)+(?:\.\*)?)[ \t]*;[ \t]*(?:\r?\n|$|(?=(?:package|import|public|protected|private|abstract|final|class|interface|enum|record)\b|@))/
    ],
    [
        "swift",
        /^(?:import[ \t]+(?:Foundation|SwiftUI|UIKit|AppKit|Swift)[ \t]*(?:\r?\n|$)|(?:public[ \t]+)?func[ \t]+\w+\s*\([^)]*\w+\s*:\s*\w+[^)]*\)\s*(?:->\s*[\w?<>[\]: ]+\s*)?\{)/
    ],
    [
        "powershell",
        /^(?:param\s*\(\s*(?:\)|(?:\[[\w.\][ ]+\])?\$\w+)|function[ \t]+\w+-\w+\s*(?:\([^)]*\)\s*)?\{|(?:Get|Set|Write|New|Remove|Import|Test|Invoke|Start|Stop)-\w+[ \t]*(?:["'$|;]|-\w|[./\\]|\r?\n|$))/i
    ],
    [
        "scss",
        /^(?:\$[\w-]+\s*:[^;{}]+;|@mixin\s+[\w-]+\s*(?:\([^)]*\)\s*)?\{|@include\s+[\w.-]+\s*(?:\([^)]*\)\s*)?[;{]|@(?:use|forward)\s+["'][^\r\n;]+;)/
    ],
    ["less", /^@[\w-]+\s*:[^;{}]+;/],
    [
        "lua",
        /^(?:--[^\n]*\n\s*)*local[ \t]+(?:function[ \t]+\w+\s*\([^)]*\)|\w+\s*=\s*(?:["'][^"'\r\n]*["'][ \t]*(?:\r?\n|$)|[\w.]+[ \t]*(?:\r?\n|$)|\w+\s*\(|\{))/
    ]
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
    const script = head.replace(/^(?:#[^\r\n]*(?:\r?\n|$)|\s)+/, "");
    for (const [language, prefix] of codePrefixes)
    {
        if (prefix.test(language === "powershell" ? script : head))
        {
            return language;
        }
    }
    if (
        /^(?:#[ \t]*include[ \t]+(?:<[^<>\s]+>|"[^"\r\n]+")|#[ \t]*pragma[ \t]+once[ \t]*(?:\r?\n|$)|(?:static[ \t]+)?(?:int|void|char|float|double)[ \t]+\w+\s*\([^)]*\)\s*[{;]|struct[ \t]+\w+\s*\{)/
            .test(head)
    )
    {
        return /\bstd::|\btemplate\s*<|\busing[ \t]+namespace\b|#[ \t]*include[ \t]*<[a-z_]+>/.test(head)
            ? "cpp"
            : "c";
    }
    if (
        /^(?:template\s*<[^>]+>\s*\w+[\s\w:*<>]*[{(]|using[ \t]+namespace[ \t]+\w+\s*;|std::\w+(?:<[^>]+>)?\s+(?:\w+\s*[=;{(]|<<))/
            .test(head)
    )
    {
        return "cpp";
    }
    const configuration = sample.replace(/^(?:\s|[#;][^\n]*(?:\n|$))+/, "");
    if (
        /^(?:ARG[ \t]+\w+(?:=[^\r\n]+)?\r?\n\s*)*FROM[ \t]+[\w./:@-]+(?:[ \t]+AS[ \t]+\w+)?[ \t]*\r?\n\s*(?:RUN|COPY|ADD|WORKDIR|CMD|ENTRYPOINT|ENV|LABEL|EXPOSE|USER|VOLUME|ARG|STOPSIGNAL|HEALTHCHECK|SHELL|ONBUILD)[ \t]+\S/i
            .test(configuration)
    )
    {
        return "dockerfile";
    }
    const configurationLanguage = detectConfiguration(sample, text.length >= DETECTION_LIMIT);
    if (configurationLanguage)
    {
        return configurationLanguage;
    }
    if (_sqlSyntax(head))
    {
        return "sql";
    }
    if (/^(?: {0,3}> ?)* {0,3}(?:`{3,}|~{3,})[^\n]*$/m.test(code))
    {
        return "markdown";
    }
    if (/^(?:require[ \t]+["']|def[ \t]+\w+[^\n:]*\n)/.test(script) && /^[ \t]*end[ \t]*(?:\r?$)/m.test(script))
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
        /^(?:<[A-Za-z]|\{[#@:/])/.test(head)
        && (svelteTemplates.some(pattern => pattern.test(code))
            || (/<script\b[^>]*>/.test(code) && svelteScript.test(sample)))
    )
    {
        return "svelte";
    }
    // A module docstring precedes Python imports and definitions. Its masked body
    // is opaque, but its delimiters must not become a document-language gate.
    const pythonHead = script.replace(/^(?:[rRuU])?(?:"[^"]*"|'[^']*')[ \t]*(?:\r?\n|$)\s*/, "")
        .replace(/^(?:#[^\r\n]*(?:\r?\n|$)|\s)+/, "");
    const pythonDocument =
        /^(?:(?:from[ \t]+[\w.]+[ \t]+import[ \t]+[\w.*(), ]+|import[ \t]+[\w., ]+)[ \t]*(?:\r?\n|$)|@\w|(?:async[ \t]+)?def[ \t]+)/
            .test(pythonHead);
    if (
        pythonDocument
        && /^[ \t]*(?:async +)?def +\w+\([^)]*\)[ \t]*(?:->[^\n:]+)?[ \t]*:[ \t]*(?:\r?\n|(?:return|raise|yield|pass)\b)/m
            .test(script)
    )
    {
        return "python";
    }
    const rustHead = head.replace(/^(?:#!?\[[^\]]*\]\s*)+/, "");
    const rustItem = /^(?:(?:pub(?:\([^)]*\))?|async|const|unsafe)\s+)*(?:fn|use|mod|struct|enum|trait|impl)\b/.test(
        rustHead
    );
    const rustDocument = rustItem
        || /^(?:pub(?:\([^)]*\))?\s+)?(?:(?:const|static(?:\s+mut)?)\s+\w+\s*:[^=;]+\s*=|type\s+\w+(?:<[^>]+>)?\s*=|extern\s+crate\s+\w+\s*;)/
            .test(rustHead);
    if (
        rustDocument
        && (/^[ \t]*(?:pub(?:\([^)]*\))?[ \t]+)?(?:(?:async|const|unsafe)[ \t]+)*fn[ \t]+\w+\s*[<(]/m.test(code)
            || /^[ \t]*(?:pub(?:\([^)]*\))?[ \t]+)?use[ \t]+\w+::[\w:{}*, \t]+;/m.test(code)
            || /^[ \t]*pub(?:\([^)]*\))?[ \t]+(?:struct|enum|trait)[ \t]+\w+\s*[<{(;]/m.test(code)
            || /^[ \t]*(?:unsafe[ \t]+)?(?:impl(?:[ \t]+|<)|trait[ \t]+)\w[^;{}]*\{/m.test(code))
    )
    {
        return "rust";
    }
    if (/^(?:<!doctype[ \t]+html[ \t]*>|<html(?:\s[^<>]*|)>)/i.test(head))
    {
        return "html";
    }
    const jsx = /<>[\s\S]*?<\/>|<[A-Za-z][\w.:-]*\b[^<>]*\/>/.test(code)
        || /<([A-Za-z][\w.:-]*)\b[^<>]*>[\s\S]*?<\/\1\s*>/i.test(code);
    // JavaScript directive prologues are string expression statements; the
    // spelling of a directive is not part of language recognition.
    const scriptHead = head.replace(/^(?:(?:"[^"\r\n]*"|'[^'\r\n]*')[ \t]*(?:;[ \t]*(?:\r?\n)?|\r?\n)\s*)+/, "");
    const scriptDocument = javascriptModules.some(pattern => pattern.exec(scriptHead)?.index === 0)
        || /^(?:(?:export|default|declare|abstract|async)\s+)*(?:import|const|let|var|function|class|interface|type|namespace|module|enum|global)\b/
            .test(scriptHead);
    if (scriptDocument && typescriptDeclarations.some(pattern => pattern.test(scriptHead)))
    {
        return jsx ? "tsx" : "typescript";
    }
    if (
        scriptDocument
        && (javascriptModules.some(pattern => pattern.test(scriptHead))
            || /^[ \t]*(?:(?:export|default|async) +)*(?:(?:const|let|var) +[$\w]+\s*=|function[ \t]*\*?[ \t]*[$\w]*\s*\([^)]*\)\s*\{)/m
                .test(scriptHead))
    )
    {
        return jsx ? "jsx" : "javascript";
    }
    if (/^<[a-z][\w-]*\b/i.test(head) && jsx)
    {
        return "html";
    }
    if (_cssSyntax(head))
    {
        return "css";
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
 * Recognize a statement, not prose containing SQL verbs. A bare SELECT/FROM pair
 * is ambiguous without a terminator, expression punctuation or another clause.
 */
function _sqlSyntax(code: string): boolean
{
    const statement = code.replace(/^(?:--[^\r\n]*(?:\r?\n|$)|\s)+/, "");
    if (sqlStatement.test(statement))
    {
        return true;
    }
    const select = sqlSelect.exec(statement);
    const projection = select?.[1];
    let tail = select?.[2];
    if (projection === undefined || tail === undefined)
    {
        return false;
    }
    tail = tail.trim();
    // An alias must not consume the clause keyword that supplies our evidence.
    if (!/^(?:WHERE|LEFT|RIGHT|INNER|OUTER|CROSS|FULL|JOIN|GROUP|ORDER|HAVING|LIMIT|UNION)\b/i.test(tail))
    {
        tail = tail.replace(sqlAlias, "");
    }
    const clause =
        /^(?:WHERE|(?:LEFT|RIGHT|INNER|OUTER|CROSS|FULL)\s+JOIN|JOIN|GROUP\s+BY|ORDER\s+BY|HAVING|LIMIT|UNION)\s+\S/i
            .test(tail);
    return clause || /^;(?:\s*(?:SELECT|INSERT|UPDATE|DELETE|CREATE|DROP|ALTER)\b|\s*$)/i.test(tail)
        || (tail === "" && /[*(),]/.test(projection));
}

/**
 * Require the first rule to have selector tokens and a declaration. Do not search
 * past prose for a later rule. Token scanning avoids backtracking over word runs.
 */
function _cssSyntax(code: string): boolean
{
    const rules = code
        .replace(/^(?:@(?:charset|import|namespace)\s+[^{};]+;\s*)+/, "")
        .replace(
            /^(?:@(?:media|supports|container|layer|scope|starting-style|keyframes|-webkit-keyframes)\b[^{};]*\{\s*)+/,
            ""
        );
    const rule = /^([^{};]+)\{\s*(?:--)?[a-zA-Z][\w-]*\s*:[^{};]+([;}])/.exec(rules);
    const selector = rule?.[1]?.trim();
    if (!rule || !selector)
    {
        return false;
    }
    if (/^@(?:font-face|page|property|counter-style)\b[\w\s:-]*$/.test(selector))
    {
        return true;
    }
    // Colons belong to adjacent pseudo names; dots and hashes need identifiers.
    const token = /(?:[.#]?[-_a-zA-Z][\w-]*|::?[\w-]+|\*|\[[^\]\r\n]+\]|\([^{}]*\)|\d+(?:\.\d+)?%|[>+~|,])\s*/y;
    let position = 0;
    while (position < selector.length)
    {
        token.lastIndex = position;
        if (!token.test(selector))
        {
            return false;
        }
        position = token.lastIndex;
    }
    // Several plain words plus an object-shaped example also occur in prose.
    return !/^[\w-]+(?:\s+[\w-]+)+$/.test(selector) || rule[2] === ";";
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
