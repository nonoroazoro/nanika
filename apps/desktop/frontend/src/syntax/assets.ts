import { parseRawGrammar } from "vscode-textmate";

import type { IRawGrammar } from "vscode-textmate";

import manifest from "./grammars/manifest.json";

// Only asset URLs enter the worker bundle. Grammar source is loaded on demand.
const urls = import.meta.glob<string>(["./grammars/*.json", "!./grammars/manifest.json"], {
    query: "?url&no-inline",
    import: "default",
    eager: true
});

/**
 * Resolve a scope exclusively against the bundled grammar allowlist.
 *
 * @param scope TextMate scope requested by another bundled grammar.
 */
export async function loadSyntaxGrammar(scope: string): Promise<IRawGrammar | null>
{
    if (!Object.hasOwn(manifest.grammars, scope))
    {
        return null;
    }
    const entry = manifest.grammars[scope as keyof typeof manifest.grammars];
    const url = urls[`./grammars/${entry.asset}`];
    if (!url)
    {
        throw new Error("Bundled syntax asset is missing.");
    }
    const response = await fetch(url);
    if (!response.ok)
    {
        throw new Error("Bundled syntax asset could not be loaded.");
    }
    return parseRawGrammar(await response.text(), entry.asset);
}

/**
 * Return the pinned injection grammars associated with a language.
 *
 * @param scope Parent grammar scope.
 */
export function syntaxInjections(scope: string): string[]
{
    return Object.entries(manifest.grammars)
        .filter(([, entry]) => (entry.injectTo as string[]).includes(scope))
        .map(([name]) => name);
}
