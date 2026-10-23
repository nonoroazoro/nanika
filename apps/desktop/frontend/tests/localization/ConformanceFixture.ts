import type { LocalizedText } from "../../src/generated/LocalizedText";
import type { MessageContract } from "../../src/generated/MessageContract";
import type { SupportedLocale } from "../../src/generated/SupportedLocale";

export interface ConformanceFixture
{
    contracts: Record<string, MessageContract>;
    cases: Array<{
        expected: string;
        locale: SupportedLocale;
        platform: "macos" | "windows";
        text: LocalizedText;
    }>;
}
