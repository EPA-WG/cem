import path from 'node:path';
import { fileURLToPath } from 'node:url';

import { defineConfig } from 'vitest/config';

const dirname = path.dirname(fileURLToPath(import.meta.url));

// Plain node unit tests (`*.spec.ts`), isolated from the browser-based
// Storybook test project in `vitest.config.mts`. Home for pure-logic units
// such as the BR-VC-9 disposition decision-core (`src/lib/disposition.ts`).
export default defineConfig({
    root: dirname,
    cacheDir: '../../node_modules/.vite/packages/cem-elements-unit',
    test: {
        name: 'unit',
        environment: 'node',
        include: ['src/**/*.spec.ts'],
    },
});
