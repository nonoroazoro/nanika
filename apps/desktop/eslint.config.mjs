import typescriptParser from "@typescript-eslint/parser";
import svelte from "eslint-plugin-svelte";
import { defineConfig } from "eslint-config-zoro";

export default [
    ...(await defineConfig({
        typescript: true,
        ignores: ["frontend/dist/**", "frontend/node_modules/**"],
        languageOptions: {
            parserOptions: {
                project: "./tsconfig.eslint.json",
                tsconfigRootDir: import.meta.dirname
            }
        }
    })),
    ...svelte.configs.recommended,
    {
        files: ["frontend/**/*.svelte"],
        languageOptions: {
            parserOptions: {
                extraFileExtensions: [".svelte"],
                parser: typescriptParser,
                project: "./tsconfig.eslint.json",
                tsconfigRootDir: import.meta.dirname
            }
        },
        rules: {
            "no-console": ["error", { allow: ["error"] }]
        }
    }
];
