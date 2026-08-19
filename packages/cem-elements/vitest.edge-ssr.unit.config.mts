import path from 'node:path';
import { fileURLToPath } from 'node:url';

import { defineConfig } from 'vitest/config';

const dirname = path.dirname(fileURLToPath(import.meta.url));

export default defineConfig({
    root: dirname,
    cacheDir: '../../node_modules/.vite/packages/cem-elements-edge-ssr-unit',
    test: {
        name: 'edge-ssr-unit',
        environment: 'node',
        include: ['src/**/*.edge-ssr.spec.ts', 'src/**/edge-ssr-*.spec.ts'],
    },
});
