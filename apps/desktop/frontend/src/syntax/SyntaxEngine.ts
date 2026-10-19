import { INITIAL, Registry } from "vscode-textmate";

import type { RegistryOptions } from "vscode-textmate";

import { LANGUAGE_SCOPES } from "./languages";
import { LINE_LENGTH_LIMIT, LINE_TIME_MS, SLICE_MS, TOKEN_BATCH_CHARS } from "./limits";
import { plainColor, SYNTAX_THEMES, TOKEN_COLORS } from "./palette";

import type { SyntaxContinuation } from "./SyntaxContinuation";
import type { SyntaxCursor } from "./SyntaxCursor";
import type { SyntaxLanguage } from "./SyntaxLanguage";
import type { SyntaxResult } from "./SyntaxResult";
import type { SyntaxTheme } from "./SyntaxTheme";

/**
 * Worker-owned TextMate engine. Callers own cancellation and publication.
 */
export class SyntaxEngine
{
    private readonly _registry: Registry;
    private _theme: SyntaxTheme = "light";
    private readonly _streams = new Map<number, SyntaxCursor>();

    constructor(options: Pick<RegistryOptions, "getInjections" | "loadGrammar" | "onigLib">)
    {
        this._registry = new Registry({ ...options, theme: SYNTAX_THEMES.light, colorMap: TOKEN_COLORS });
    }

    async append(
        text: string,
        language: SyntaxLanguage,
        signal: AbortSignal,
        theme: SyntaxTheme,
        continuation: SyntaxContinuation
    ): Promise<SyntaxResult>
    {
        signal.throwIfAborted();
        if (text.length > TOKEN_BATCH_CHARS)
        {
            throw new Error("Syntax batch exceeds its input budget.");
        }
        if (this._theme !== theme)
        {
            this._registry.setTheme(SYNTAX_THEMES[theme], TOKEN_COLORS);
            this._theme = theme;
        }
        const previous = this._streams.get(continuation.stream);
        if (
            previous
                ? previous.offset !== continuation.offset || previous.language !== language || previous.theme !== theme
                : continuation.offset !== 0
        )
        {
            throw new Error("Syntax stream is no longer available.");
        }
        if (!previous && this._streams.size >= 8)
        {
            throw new Error("Too many active syntax streams.");
        }
        const cursor: SyntaxCursor = previous
            ? { ...previous }
            : {
                offset: 0,
                lineStart: 0,
                pending: "",
                skipping: false,
                state: INITIAL,
                language,
                theme
            };
        const grammar = await this._registry.loadGrammar(LANGUAGE_SCOPES[language]);
        signal.throwIfAborted();
        if (!grammar)
        {
            throw new Error(`Missing bundled syntax grammar: ${language}`);
        }
        const ranges: number[] = [];
        let start = 0;
        let slice = performance.now();
        while (start < text.length)
        {
            signal.throwIfAborted();
            const newline = text.indexOf("\n", start);
            const end = newline < 0 ? text.length : newline;
            if (!cursor.skipping)
            {
                if (cursor.pending.length + end - start > LINE_LENGTH_LIMIT)
                {
                    cursor.pending = "";
                    cursor.skipping = true;
                }
                else
                {
                    cursor.pending += text.slice(start, end);
                }
            }
            cursor.offset += end - start;
            if (newline >= 0 || continuation.final)
            {
                if (!cursor.skipping)
                {
                    const line = cursor.pending.endsWith("\r") ? cursor.pending.slice(0, -1) : cursor.pending;
                    const result = line.length < LINE_LENGTH_LIMIT
                        ? grammar.tokenizeLine2(line, cursor.state, LINE_TIME_MS)
                        : null;
                    // Like VS Code, a pathological line stays plain without discarding
                    // the surrounding document or advancing an incomplete grammar state.
                    if (result && !result.stoppedEarly)
                    {
                        cursor.state = result.ruleStack;
                        this._append(ranges, result.tokens, cursor.lineStart, cursor.lineStart + line.length);
                    }
                }
                cursor.offset += newline >= 0 ? 1 : 0;
                cursor.lineStart = cursor.offset;
                cursor.pending = "";
                cursor.skipping = false;
            }
            start = end + 1;
            if (performance.now() - slice >= SLICE_MS)
            {
                await new Promise<void>(resolve =>
                {
                    setTimeout(resolve, 0);
                });
                slice = performance.now();
            }
        }
        signal.throwIfAborted();
        if (continuation.final)
        {
            this._streams.delete(continuation.stream);
        }
        else
        {
            this._streams.set(continuation.stream, cursor);
        }
        return { language, ranges: Uint32Array.from(ranges) };
    }

    release(stream: number): void
    {
        this._streams.delete(stream);
    }

    dispose(): void
    {
        this._streams.clear();
        this._registry.dispose();
    }

    private _append(ranges: number[], tokens: Uint32Array, offset: number, end: number): void
    {
        for (let index = 0; index < tokens.length; index += 2)
        {
            // TextMate's encoded-token contract puts foreground in bits 15..23.
            const color = ((tokens[index + 1] ?? 0) >>> 15) & 511;
            const start = offset + (tokens[index] ?? 0);
            const stop = Math.min(end, index + 2 < tokens.length ? offset + (tokens[index + 2] ?? 0) : end);
            if (color === 0 || color === plainColor(this._theme) || start >= stop)
            {
                continue;
            }
            if (ranges.at(-1) === color && ranges.at(-2) === start)
            {
                ranges[ranges.length - 2] = stop;
            }
            else
            {
                ranges.push(start, stop, color);
            }
        }
    }
}
