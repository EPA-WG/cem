import assert from 'node:assert/strict';
import { rm } from 'node:fs/promises';
import { dirname, join, parse } from 'node:path';
import { fileURLToPath } from 'node:url';

const projectRoot = dirname(dirname(fileURLToPath(import.meta.url)));
const distRoot = join(projectRoot, 'dist');

assert.equal(dirname(distRoot), projectRoot, 'dist output must be a direct child of the project root');
assert.equal(parse(distRoot).base, 'dist', 'only the generated dist directory may be removed');

await rm(distRoot, { recursive: true, force: true });
console.log('Removed stale cem-elements dist output.');
