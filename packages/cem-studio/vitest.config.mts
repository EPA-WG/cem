import { playwright } from '@vitest/browser-playwright';
import { defineConfig } from 'vitest/config';
import { browserLoadDiagnosticCommand } from './scripts/browser-load-diagnostics.mjs';

const loadDiagnostics = process.env.CEM_STUDIO_LOAD_DIAGNOSTICS;

export default defineConfig({
    root: __dirname,
    cacheDir: process.env.CEM_STUDIO_TEST_CACHE_DIR ?? '../../node_modules/.vite/packages/cem-studio',
    test: {
        globals: true,
        projects: [
            {
                extends: true,
                test: {
                    name: 'browser',
                    setupFiles: loadDiagnostics ? ['./scripts/browser-load-diagnostics.setup.mjs'] : [],
                    include: ['src/**/*.browser.{test,spec}.{js,mjs,cjs,ts,mts,cts}'],
                    browser: {
                        enabled: true,
                        headless: true,
                        provider: playwright({ launchOptions: { executablePath: process.env.CEM_STUDIO_CHROME } }),
                        commands: loadDiagnostics ? {
                            startCemBrowserLoadDiagnostics: browserLoadDiagnosticCommand(loadDiagnostics),
                        } : {},
                        instances: [{ browser: 'chromium' }],
                    },
                },
            },
        ],
    },
});
