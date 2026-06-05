import { cp, mkdir, readFile, rm, writeFile } from 'node:fs/promises';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const projectRoot = dirname(dirname(fileURLToPath(import.meta.url)));
const distRoot = join(projectRoot, 'dist');

const entries = [
    'LICENSE',
    'README.md',
    'custom-element.d.ts',
    'custom-element.js',
    'datasource.md',
    'http-request.js',
    'index.html',
    'index.js',
    'local-storage.js',
    'location-element.js',
    'module-url.js',
    'package.json',
    'bin',
    'demo',
    'docs',
    'ide',
];

await rm(distRoot, { recursive: true, force: true });
await mkdir(distRoot, { recursive: true });

for (const entry of entries) {
    await cp(join(projectRoot, entry), join(distRoot, entry), { recursive: true });
}

const packageJsonPath = join(distRoot, 'package.json');
const packageJson = JSON.parse(await readFile(packageJsonPath, 'utf8'));
packageJson.scripts = {
    ...packageJson.scripts,
    test: packageJson.scripts?.test ?? 'echo "custom-element package smoke tests are defined in the workspace"',
};
await writeFile(packageJsonPath, `${JSON.stringify(packageJson, null, 4)}\n`);
