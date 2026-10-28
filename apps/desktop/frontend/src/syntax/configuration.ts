/**
 * Recognize configuration structure in an already bounded source sample.
 * Partial final statements are not evidence against earlier complete statements.
 *
 * @param sample Bounded original text, with quotes and comments intact
 * @param truncated Whether the sampling boundary may cut a statement
 */
export function detectConfiguration(sample: string, truncated: boolean): "ini" | "toml" | "yaml" | null
{
    const head = sample.replace(/^(?:\s|[#;][^\n]*(?:\n|$))+/, "");
    return _iniOrToml(head, truncated) ?? (_yaml(sample, truncated) ? "yaml" : null);
}

/**
 * INI values may contain arbitrary unquoted prose. Only a value that starts with
 * a quote or collection delimiter enters TOML's multiline value scanner.
 */
function _iniOrToml(sample: string, truncated: boolean): "ini" | "toml" | null
{
    if (!/^(?:\[|(?:[\w.-]+|"[^"\r\n]+"|'[^'\r\n]+')\s*=)/.test(sample))
    {
        return null;
    }
    let position = 0;
    let sections = 0;
    let assignments = 0;
    let typed = 0;
    while (position < sample.length)
    {
        let end = sample.indexOf("\n", position);
        if (end < 0)
        {
            end = sample.length;
        }
        const line = sample.slice(position, end).trim();
        if (!line || /^[#;]/.test(line))
        {
            position = end + 1;
            continue;
        }
        if (/^(?:\[\[[^\]\r\n]+\]\]|\[[^\]\r\n]+\])[ \t]*(?:[#;].*)?$/.test(line))
        {
            sections++;
            position = end + 1;
            continue;
        }
        const assignment = /^[ \t]*(?:[\w.-]+|"(?:\\.|[^"\\\r\n])+"|'[^'\r\n]+')[ \t]*=[ \t]*/.exec(
            sample.slice(position)
        );
        if (!assignment)
        {
            if (truncated && end === sample.length)
            {
                break;
            }
            return null;
        }
        const start = position + assignment[0].length;
        const valueStart = sample.charAt(start);
        if (valueStart && "\"'[{".includes(valueStart))
        {
            const value = _valueEnd(sample, start);
            if (!value || (!value.complete && !truncated))
            {
                return null;
            }
            const nextLine = sample.indexOf("\n", value.end);
            const statementEnd = nextLine < 0 ? sample.length : nextLine;
            const tail = sample.slice(value.end, statementEnd).trim();
            if (!tail || /^[#;]/.test(tail))
            {
                typed++;
            }
            else if (value.end > end)
            {
                // INI prose is a single physical line, not a multiline TOML value.
                return null;
            }
            assignments++;
            position = statementEnd + 1;
            continue;
        }
        if (truncated && end === sample.length)
        {
            // A cut key, number or bare value cannot revoke earlier evidence.
            break;
        }
        const value = sample.slice(start, end).trim().replace(/(?:^|[ \t]+)[#;].*$/, "")
            .trim();
        if (!value)
        {
            return null;
        }
        assignments++;
        if (/^(?:true|false|[+-]?(?:inf|nan|0[xob][\da-fA-F_]+|\d[\d_.eE:+TZ-]*))$/.test(value))
        {
            typed++;
        }
        position = end + 1;
    }
    if (sections > 0 && assignments > 0)
    {
        return typed === assignments ? "toml" : "ini";
    }
    return assignments >= 2 && typed === assignments ? "toml" : null;
}

/**
 * Locate a quoted value or flow collection without interpreting its payload. The
 * matching delimiter stack distinguishes incomplete samples from mismatched pairs.
 */
function _valueEnd(source: string, start: number): { complete: boolean; end: number; } | null
{
    if (/^["']$/.test(source.charAt(start)))
    {
        return _quotedEnd(source, start);
    }
    const closing = [source.charAt(start) === "[" ? "]" : "}"];
    for (let position = start + 1; position < source.length; position++)
    {
        const character = source.charAt(position);
        if (character === '"' || character === "'")
        {
            const quoted = _quotedEnd(source, position);
            if (!quoted?.complete)
            {
                return quoted;
            }
            position = quoted.end - 1;
        }
        else if (character === "#")
        {
            const newline = source.indexOf("\n", position);
            position = newline < 0 ? source.length : newline;
        }
        else if (character === "[" || character === "{")
        {
            closing.push(character === "[" ? "]" : "}");
        }
        else if (character === "]" || character === "}")
        {
            if (closing.pop() !== character)
            {
                return null;
            }
            if (closing.length === 0)
            {
                return { complete: true, end: position + 1 };
            }
        }
    }
    return { complete: false, end: source.length };
}

/**
 * TOML basic strings escape with backslashes; literal strings do not. Triple
 * delimiters permit multiline values, without treating their contents as code.
 */
function _quotedEnd(source: string, start: number): { complete: boolean; end: number; } | null
{
    const quote = source.charAt(start);
    const delimiter = source.startsWith(quote.repeat(3), start) ? quote.repeat(3) : quote;
    for (let position = start + delimiter.length; position < source.length; position++)
    {
        if (delimiter.length === 1 && /[\r\n]/.test(source.charAt(position)))
        {
            return null;
        }
        if (quote === '"' && source.charAt(position) === "\\")
        {
            if (delimiter.length === 1 && /[\r\n]/.test(source.charAt(position + 1)))
            {
                return null;
            }
            position++;
        }
        else if (source.startsWith(delimiter, position))
        {
            let end = position + delimiter.length;
            // A multiline string may end with one or two literal quote characters.
            if (delimiter.length === 3)
            {
                while (end < position + 5 && source.charAt(end) === quote)
                {
                    end++;
                }
            }
            return { complete: true, end };
        }
    }
    return { complete: false, end: source.length };
}

/**
 * Recognize common YAML shapes, not the complete grammar. Require coherent
 * lines and independent structure so ordinary label/value reports stay plain.
 */
function _yaml(sample: string, truncated: boolean): boolean
{
    const source = truncated ? sample.slice(0, sample.lastIndexOf("\n") + 1) : sample;
    const indents = [0];
    let position = 0;
    let mappings = 0;
    let sequences = 0;
    let nested = 0;
    let typed = 0;
    let explicit = false;
    let flow = false;
    let block = false;
    let blockIndent: number | null = null;
    let mayNest = false;
    while (position < source.length)
    {
        const newline = source.indexOf("\n", position);
        const end = newline < 0 ? source.length : newline;
        const raw = source.slice(position, end);
        const content = raw.trim();
        const indent = raw.length - raw.trimStart().length;
        const start = position + indent;
        position = end + 1;
        if (!content)
        {
            continue;
        }
        if (blockIndent !== null && indent > blockIndent)
        {
            block = true;
            continue;
        }
        blockIndent = null;
        if (content.startsWith("#"))
        {
            continue;
        }
        if (content === "---" && mappings + sequences === 0)
        {
            explicit = true;
            continue;
        }
        if (raw.slice(0, indent).includes("\t"))
        {
            return false;
        }
        if (indent > (indents.at(-1) ?? 0))
        {
            if (!mayNest)
            {
                return false;
            }
            indents.push(indent);
        }
        while (indent < (indents.at(-1) ?? 0))
        {
            indents.pop();
        }
        if (indent !== indents[indents.length - 1])
        {
            return false;
        }
        const sequence = /^-[ \t]+/.exec(content);
        const entry = content.slice(sequence?.[0].length ?? 0);
        const mapping = /^(?:[\w.-]+|"[^"\r\n]+"|'(?:''|[^'\r\n])+'):(?:[ \t]+|$)/.exec(entry);
        if (!sequence && !mapping)
        {
            return false;
        }
        mappings += Number(Boolean(mapping));
        sequences += Number(Boolean(sequence));
        nested += Number(indent > 0);
        const value = entry.slice(mapping?.[0].length ?? 0);
        mayNest = !value || Boolean(sequence && mapping);
        if (/^[|>](?:[+-]?[1-9]?|[1-9][+-]?)(?:[ \t]+#.*)?$/.test(value))
        {
            blockIndent = indent;
            continue;
        }
        // Anchors are only lexical prefixes; detection never resolves aliases.
        const anchor = /^&[\w-]+[ \t]+/.exec(value)?.[0] ?? "";
        const scalar = value.slice(anchor.length);
        if (/^[[{]/.test(scalar))
        {
            const offset = start + content.length - scalar.length;
            const collection = _valueEnd(source, offset);
            if (!collection || (!collection.complete && !truncated))
            {
                return false;
            }
            const nextLine = source.indexOf("\n", collection.end);
            const statementEnd = nextLine < 0 ? source.length : nextLine;
            if (!/^(?:#.*)?$/.test(source.slice(collection.end, statementEnd).trim()))
            {
                return false;
            }
            flow = flow || (collection.complete && collection.end > offset + 2);
            position = statementEnd + 1;
        }
        else if (
            /^(?:true|false|null|~|[+-]?\d+(?:\.\d+)?)(?:[ \t]+#.*)?$/.test(scalar)
            || /^(?:"(?:\\.|[^"\\])*"|'(?:''|[^'])*')(?:[ \t]+#.*)?$/.test(scalar)
        )
        {
            typed++;
        }
    }
    return block || typed >= 2 || nested >= 2 || (mappings > 0 && (flow || sequences >= 2))
        || (explicit && mappings + sequences > 0);
}
