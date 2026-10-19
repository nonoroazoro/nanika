import type { IRawTheme } from "vscode-textmate";

import dark from "./themes/dark.json";
import light from "./themes/light.json";

import type { SyntaxTheme } from "./SyntaxTheme";

export const SYNTAX_FOREGROUNDS: Record<SyntaxTheme, string> = { light: _foreground(light), dark: _foreground(dark) };
export const SYNTAX_THEMES: Record<SyntaxTheme, IRawTheme> = { light: _theme(light), dark: _theme(dark) };
// One color map gives worker results stable IDs across both themes. Index zero
// is reserved by TextMate. Default foreground ranges inherit the preview color.
export const TOKEN_COLORS = [
    "",
    ...new Set(
        Object.values(SYNTAX_THEMES).flatMap(theme =>
        {
            return theme.settings.flatMap(rule => (rule.settings.foreground ? [rule.settings.foreground] : []));
        })
    )
];

export function plainColor(theme: SyntaxTheme): number
{
    return TOKEN_COLORS.indexOf(SYNTAX_FOREGROUNDS[theme]);
}

function _foreground(theme: IRawTheme): string
{
    const foreground = theme.settings[0]?.settings.foreground;
    if (!foreground)
    {
        throw new Error(`Missing default foreground for ${theme.name}.`);
    }
    return foreground;
}

function _theme(source: IRawTheme): IRawTheme
{
    const foreground = _foreground(source);
    return {
        ...source,
        settings: [{ settings: { foreground, background: foreground } }, ...source.settings.slice(1)]
    };
}
