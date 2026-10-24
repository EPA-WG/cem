import { fileURLToPath } from 'node:url';
import { defineConfig } from 'vitest/config';

// Explicit optional-profile integration cohort. Ordinary unit tests do not
// acquire an optional native build as an implicit prerequisite.
export default defineConfig({
    root: fileURLToPath(new URL('.', import.meta.url)),
    test: { name: 'schema-assets', environment: 'node', include: ['src/**/*.schema-assets.spec.ts', 'src/lib/http-resource-transport.spec.ts'] },
});
