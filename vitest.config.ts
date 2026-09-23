import { defineConfig } from "vitest/config";

export default defineConfig({
    test: {
        projects: [
            {
                extends: "./apps/desktop/frontend/vite.config.ts",
                root: "./apps/desktop/frontend",
                test: {
                    name: "frontend",
                    include: ["tests/**/*.test.ts"]
                }
            },
            {
                test: {
                    name: "tooling",
                    include: ["tooling/**/*.test.ts"]
                }
            }
        ]
    }
});
