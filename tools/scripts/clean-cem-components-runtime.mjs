import { rm } from 'node:fs/promises';
import { resolve } from 'node:path';

// TypeScript incremental builds do not remove outputs for retired source files.
// Leave independently built styles, catalogs and playgrounds intact.
const root = resolve(import.meta.dirname, '../../packages/cem-components/dist');
await Promise.all(['lib', 'index.js', 'index.d.ts', 'index.d.ts.map', 'tsconfig.lib.tsbuildinfo']
    .map(path => rm(resolve(root, path), { recursive: true, force: true })));
console.log('Removed stale cem-components runtime output.');
