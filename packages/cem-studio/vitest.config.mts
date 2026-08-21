import { playwright } from '@vitest/browser-playwright';
import { defineConfig } from 'vitest/config';

export default defineConfig({
    root: __dirname,
    cacheDir: '../../node_modules/.vite/packages/cem-studio',
    test: {
        globals: true,
        projects: [
            {
                extends: true,
                test: {
                    name: 'browser',
                    include: ['src/**/*.browser.{test,spec}.{js,mjs,cjs,ts,mts,cts}'],
                    browser: {
                        enabled: true,
                        headless: true,
                        provider: playwright({}),
                        instances: [{ browser: 'chromium' }],
                    },
                },
            },
        ],
    },
});
