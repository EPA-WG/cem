import path from 'node:path';
import { fileURLToPath } from 'node:url';

import { storybookTest } from '@storybook/addon-vitest/vitest-plugin';
import { playwright } from '@vitest/browser-playwright';
import { defineConfig } from 'vitest/config';

const dirname = path.dirname(fileURLToPath(import.meta.url));

export default defineConfig({
    root: dirname,
    cacheDir: '../../node_modules/.vite/packages/cem-elements-edge-ssr',
    test: {
        projects: [
            {
                extends: true,
                plugins: [
                    storybookTest({
                        configDir: path.join(dirname, '.storybook-edge-ssr'),
                        storybookScript:
                            'yarn storybook dev --config-dir packages/cem-elements/.storybook-edge-ssr --host 127.0.0.1 --port 4401 --no-open',
                    }),
                ],
                test: {
                    name: 'edge-ssr-storybook',
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
