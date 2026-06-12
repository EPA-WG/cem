import { cp, mkdir, readFile, rm, writeFile } from 'node:fs/promises';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const projectRoot = dirname(dirname(fileURLToPath(import.meta.url)));
const workspaceRoot = dirname(dirname(projectRoot));
const distRoot = join(projectRoot, 'dist');
const vendorRoot = join(distRoot, 'vendor/@epa-wg');

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

await cp(
    join(workspaceRoot, 'packages/cem-elements/dist'),
    join(vendorRoot, 'cem-elements/dist'),
    // Drop incremental-build artifacts; the vendored runtime is JS/WASM only.
    { recursive: true, filter: (src) => !src.endsWith('.tsbuildinfo') }
);
await cp(
    join(workspaceRoot, 'packages/cem_ql/dist/wasm'),
    join(vendorRoot, 'cem_ql/dist/wasm'),
    { recursive: true }
);

const customElementPath = join(distRoot, 'custom-element.js');
const customElementSource = await readFile(customElementPath, 'utf8');
await writeFile(
    customElementPath,
    customElementSource.replace(
        "from '../cem-elements/dist/index.js'",
        "from './vendor/@epa-wg/cem-elements/dist/index.js'"
    )
);

const packageJsonPath = join(distRoot, 'package.json');
const packageJson = JSON.parse(await readFile(packageJsonPath, 'utf8'));
packageJson.scripts = {
    ...packageJson.scripts,
    test: packageJson.scripts?.test ?? 'echo "custom-element package smoke tests are defined in the workspace"',
};
await writeFile(packageJsonPath, `${JSON.stringify(packageJson, null, 4)}\n`);
