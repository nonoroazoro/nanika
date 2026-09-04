import { svelte } from '@sveltejs/vite-plugin-svelte'
import { playwright } from '@vitest/browser-playwright'
import { defineConfig } from 'vitest/config'

export default defineConfig({
  root: import.meta.dirname,
  plugins: [svelte()],
  test: {
    projects: [
      {
        extends: false,
        test: {
          name: 'unit',
          environment: 'node',
          include: ['tests/**/*.unit.test.ts'],
        },
      },
      {
        extends: false,
        plugins: [svelte()],
        test: {
          name: 'browser',
          include: ['tests/**/*.test.ts'],
          exclude: ['tests/**/*.unit.test.ts'],
          browser: {
            enabled: true,
            headless: true,
            provider: playwright(),
            instances: [{ browser: 'chromium' }, { browser: 'webkit' }],
          },
        },
      },
    ],
  },
})
