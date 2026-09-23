import typescriptParser from "@typescript-eslint/parser";
import { defineConfig } from "eslint-config-zoro";
import svelte from "eslint-plugin-svelte";

export default [
    ...(await defineConfig({
        typescript: true,
        ignores: ["apps/desktop/frontend/dist/**", "target/**"],
        languageOptions: {
            parserOptions: {
                extraFileExtensions: [".svelte"],
                projectService: {
                    allowDefaultProject: ["apps/desktop/frontend/svelte.config.js"]
                },
                tsconfigRootDir: import.meta.dirname
            }
        }
    })),
    ...svelte.configs.recommended,
    {
        files: ["apps/desktop/frontend/src/**/*.{ts,svelte}"],
        rules: {
            "no-restricted-globals": ["error", {
                name: "Bun",
                message: "Bun is a build tool, not an application runtime."
            }],
            "no-restricted-imports": ["error", {
                patterns: [{
                    group: ["bun", "bun:*", "node:*"],
                    message: "Application code must use browser and Tauri APIs."
                }]
            }]
        }
    },
    {
        files: ["tooling/**/*.ts", "*.config.ts", "apps/desktop/frontend/vite.config.ts"],
        rules: {
            "no-console": ["error", { allow: ["log"] }]
        }
    },
    {
        files: ["apps/desktop/frontend/**/*.svelte"],
        languageOptions: {
            parserOptions: {
                parser: typescriptParser
            }
        },
        rules: {
            "no-console": ["error", { allow: ["error"] }]
        }
    }
];
