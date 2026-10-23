import { expect, test } from "vitest";

import { formatBytes } from "../../src/localization/bytes";

for (const locale of ["en", "zh-Hans"] as const)
{
    test(`${locale} formats byte boundaries as locale numbers and IEC units`, () =>
    {
        for (
            const [bytes, expected] of [
                [0, "0 B"],
                [1, "1 B"],
                [1023, "1,023 B"],
                [1024, "1 KiB"],
                [1536, "1.5 KiB"],
                [1048576, "1 MiB"],
                [1073741824, "1 GiB"],
                [1099511627776, "1 TiB"],
                [1125899906842624, "1 PiB"],
                [Number.MAX_SAFE_INTEGER, "8 PiB"]
            ] as const
        )
        {
            expect(formatBytes(bytes, locale)).toBe(expected);
        }
    });
}

test("byte formatting rejects values that the host cannot admit", () =>
{
    for (const bytes of [-1, 0.5, Number.NaN, Number.POSITIVE_INFINITY, Number.MAX_SAFE_INTEGER + 1])
    {
        expect(() => formatBytes(bytes, "en")).toThrow("Invalid metadata byte count");
    }
});
