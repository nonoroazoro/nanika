import { test } from "vitest";

import { detectSyntax } from "../../src/syntax/detect";
import { DETECTION_LIMIT } from "../../src/syntax/limits";

const css = (await Bun.file(new URL("../../src/styles/global.css", import.meta.url)).text()).slice(0, DETECTION_LIMIT);
const markdown = (await Bun.file(new URL("../../../../../THIRD_PARTY_NOTICES.md", import.meta.url)).text()).slice(
    0,
    DETECTION_LIMIT
);

test("bounded language detection", async ({ bench }) =>
{
    for (
        const [name, source] of [["CSS", css], ["Markdown", markdown], [
            "plain prose",
            "A short note for tomorrow's meeting."
        ]] as const
    )
    {
        await bench(name, {
            writeResult: process.env.NANIKA_BENCHMARK_OUTPUT
                ? `${process.env.NANIKA_BENCHMARK_OUTPUT}/${name}.json`
                : undefined
        }, () =>
        {
            detectSyntax(source);
        }).run();
    }
});
