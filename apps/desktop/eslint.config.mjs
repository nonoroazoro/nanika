import typescriptParser from "@typescript-eslint/parser";
import svelte from "eslint-plugin-svelte";
import { defineConfig } from "eslint-config-zoro";

// The pinned ESLint requires an extra loader or an experimental flag for TypeScript configuration.
export default [
    ...(await defineConfig({
        typescript: true,
        ignores: ["frontend/dist/**", "frontend/node_modules/**"],
        languageOptions: {
            parserOptions: {
                extraFileExtensions: [".svelte"],
                project: "./tsconfig.eslint.json",
                tsconfigRootDir: import.meta.dirname
            }
        }
    })),
    ...svelte.configs.recommended,
    {
        files: ["tooling/**/*.ts"],
        languageOptions: {
            parserOptions: {
                project: "./tsconfig.tooling.json",
                tsconfigRootDir: import.meta.dirname
            }
        },
        rules: {
            "no-console": ["error", { allow: ["log"] }]
        }
    },
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
