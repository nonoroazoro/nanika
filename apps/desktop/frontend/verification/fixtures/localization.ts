import applicationEn from "../../../../extensions/built-in/application/locales/en.ftl?raw";
import applicationZh from "../../../../extensions/built-in/application/locales/zh-Hans.ftl?raw";
import en from "../../../../extensions/built-in/clipboard/locales/en.ftl?raw";
import zh from "../../../../extensions/built-in/clipboard/locales/zh-Hans.ftl?raw";
import systemEn from "../../../../extensions/built-in/system/locales/en.ftl?raw";
import systemZh from "../../../../extensions/built-in/system/locales/zh-Hans.ftl?raw";

import type { ExtensionResourceScope } from "../../src/generated/ExtensionResourceScope";
import type { PreparedResources } from "../../src/generated/PreparedResources";

export const clipboardScope: ExtensionResourceScope = {
    extensionId: "com.nanika.clipboard",
    identity: "a".repeat(64)
};

export const systemScope: ExtensionResourceScope = {
    extensionId: "com.nanika.system",
    identity: "b".repeat(64)
};

export const applicationScope: ExtensionResourceScope = {
    extensionId: "com.nanika.application",
    identity: "c".repeat(64)
};

export const applicationResources = _resources(applicationScope, applicationEn, applicationZh);
export const clipboardResources = _resources(clipboardScope, en, zh);
export const systemResources = _resources(systemScope, systemEn, systemZh);

let failNextRead = false;

export function failNextLocalizationRead(): void
{
    failNextRead = true;
}

export function readFixtureResources(scope: ExtensionResourceScope): PreparedResources
{
    document.documentElement.dataset.localizationReads = String(
        Number(document.documentElement.dataset.localizationReads ?? 0) + 1
    );
    if (failNextRead)
    {
        failNextRead = false;
        throw Object.assign(new Error("Fixture resource read failed."), {
            kind: "host",
            code: "localizationPackageChanged",
            detail: "Original resource <diagnostic>"
        });
    }
    for (
        const [owner, resources] of [[clipboardScope, clipboardResources], [systemScope, systemResources], [
            applicationScope,
            applicationResources
        ]] as const
    )
    {
        if (scope.extensionId === owner.extensionId && scope.identity === owner.identity)
        {
            return resources;
        }
    }
    throw new Error("Unknown fixture resource owner.");
}

function _resources(scope: ExtensionResourceScope, english: string, chinese: string): PreparedResources
{
    // Fixture messages use direct variables; production contracts come from Rust admission.
    const locale = (source: string) => ({
        sources: [source],
        contracts: Object.fromEntries(
            [...source.matchAll(/^([a-z][a-z0-9-]*)[ \t]*=([^\n]*(?:\n[ \t]+[^\n]*)*)/gm)].map(match => [
                match[1],
                {
                    arguments: [
                        ...new Set(
                            [...(match[2] ?? "").matchAll(/\$([a-zA-Z][a-zA-Z0-9_-]*)/g)].map(arg => arg[1] ?? "")
                        )
                    ]
                        .sort()
                }
            ])
        )
    });
    return {
        identity: scope.identity,
        defaultLocale: "en",
        locales: { en: locale(english), "zh-Hans": locale(chinese) }
    };
}
