import path from 'node:path';
import { fileURLToPath } from 'node:url';

import { storybookTest } from '@storybook/addon-vitest/vitest-plugin';
import { playwright } from '@vitest/browser-playwright';
import { defineConfig } from 'vitest/config';

const dirname = path.dirname(fileURLToPath(import.meta.url));

export default defineConfig({
    root: dirname,
    cacheDir: '../../node_modules/.vite/packages/cem-demo-element',
    test: {
        projects: [
            {
                extends: true,
                plugins: [
                    storybookTest({
                        configDir: path.join(dirname, '.storybook'),
                        storybookScript:
                            'yarn storybook dev --config-dir packages/cem-demo-element/.storybook --host 127.0.0.1 --port 4401 --no-open',
                    }),
                ],
                test: {
                    name: 'storybook',
                    testTimeout: 30_000,
                    browser: {
                        enabled: true,
                        headless: true,
                        provider: playwright({}),
                        instances: [{ browser: 'chromium' }],
                    },
                    reporters: ['default'],
                },
            },
        ],
    },
});
