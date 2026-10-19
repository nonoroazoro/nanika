import { defineConfig } from "vitest/config";

export default defineConfig({
    test: {
        projects: [
            {
                extends: "./apps/desktop/frontend/vite.config.ts",
                root: "./apps/desktop/frontend",
                test: {
                    name: "frontend",
                    include: ["tests/**/*.test.ts"],
                    benchmark: { include: [] }
                }
            },
            {
                test: {
                    name: "benchmarks",
                    include: [],
                    benchmark: { include: ["apps/desktop/frontend/benchmarks/**/*.bench.ts"] }
                }
            },
            {
                root: "./apps/desktop/shell",
                test: {
                    name: "shell",
                    include: ["tests/**/*.test.ts"],
                    benchmark: { include: [] }
                }
            },
            {
                root: "./apps/desktop/verification",
                test: {
                    name: "verification",
                    include: ["tests/**/*.test.ts"],
                    benchmark: { include: [] }
                }
            },
            {
                test: {
                    name: "tooling",
                    include: ["tooling/tests/**/*.test.ts"],
                    benchmark: { include: [] }
                }
            }
        ]
    }
});
