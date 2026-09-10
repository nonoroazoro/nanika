import { svelte } from "@sveltejs/vite-plugin-svelte";
import { defineConfig } from "vite";

export default defineConfig({
    root: import.meta.dirname,
    plugins: [svelte(), {
        name: "exclude-development-monitor",
        apply: "build",
        generateBundle(_options, bundle)
        {
            for (const output of Object.values(bundle))
            {
                if (
                    output.type === "chunk"
                    && Object.keys(output.modules).some(id => id.replaceAll("\\", "/").includes("/src/development/"))
                )
                {
                    this.error("Development monitoring must not enter production assets.");
                }
            }
        }
    }],
    clearScreen: false,
    server: {
        port: 1420,
        strictPort: true,
        watch: {
            ignored: ["**/shell/**"]
        }
    },
    build: {
        target: "es2022",
        sourcemap: false
    }
});
