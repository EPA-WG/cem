import { cp, mkdir, readFile, rm, writeFile } from 'node:fs/promises';
import { dirname, join } from 'node:path';
import { fileURLToPath } from 'node:url';

const projectRoot = dirname(dirname(fileURLToPath(import.meta.url)));
const workspaceRoot = dirname(dirname(projectRoot));
const distRoot = join(projectRoot, 'dist');
const vendorRoot = join(distRoot, 'vendor/@epa-wg');
const archiveManifest = JSON.parse(await readFile(join(projectRoot, 'package-archive.json'), 'utf8'));

if (archiveManifest.schemaVersion !== 1) {
    throw new Error(`unsupported package archive manifest schema ${archiveManifest.schemaVersion}`);
}
const entries = archiveManifest.sourceEntries;
if (!Array.isArray(entries) || entries.length === 0 || new Set(entries).size !== entries.length) {
    throw new Error('package archive sourceEntries must be a non-empty unique array');
}
if (!Array.isArray(archiveManifest.packageFiles) || archiveManifest.packageFiles.length === 0) {
    throw new Error('package archive packageFiles must be a non-empty array');
}

await rm(distRoot, { recursive: true, force: true });
await mkdir(distRoot, { recursive: true });

for (const entry of entries) {
    await cp(join(projectRoot, entry), join(distRoot, entry), { recursive: true });
}

await cp(
    join(workspaceRoot, 'packages/cem-elements/dist'),
    join(vendorRoot, 'cem-elements/dist'),
    // Drop incremental-build artifacts; the vendored runtime is JS/WASM only.
    { recursive: true, filter: (src) => !src.endsWith('.tsbuildinfo') },
);
await cp(join(workspaceRoot, 'packages/cem_ql/dist/wasm'), join(vendorRoot, 'cem_ql/dist/wasm'), { recursive: true });

const customElementPath = join(distRoot, 'custom-element.js');
const customElementSource = await readFile(customElementPath, 'utf8');
await writeFile(
    customElementPath,
    customElementSource.replace(
        "from '../cem-elements/dist/index.js'",
        "from './vendor/@epa-wg/cem-elements/dist/index.js'",
    ),
);

const packageJsonPath = join(distRoot, 'package.json');
const packageJson = JSON.parse(await readFile(packageJsonPath, 'utf8'));
if (JSON.stringify(packageJson.files) !== JSON.stringify(archiveManifest.packageFiles)) {
    throw new Error('package.json files must exactly match package-archive.json packageFiles');
}
packageJson.files = [...archiveManifest.packageFiles];
delete packageJson.scripts;
await writeFile(packageJsonPath, `${JSON.stringify(packageJson, null, 4)}\n`);
