import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { mkdir, readFile, rm, writeFile } from 'node:fs/promises';
import { dirname, resolve } from 'node:path';

const projectRoot = resolve(import.meta.dirname, '..');
const workspaceRoot = resolve(projectRoot, '../..');
const outputRoot = resolve(workspaceRoot, 'dist/packages/cem-studio');
const archivePath = resolve(outputRoot, 'package.tgz');
const reportPath = resolve(outputRoot, 'package.json');
const metadata = JSON.parse(await readFile(resolve(projectRoot, 'package.json'), 'utf8'));

await mkdir(outputRoot, { recursive: true });
await rm(archivePath, { force: true });
const result = spawnSync(process.platform === 'win32' ? 'yarn.cmd' : 'yarn', ['pack', '--out', archivePath], {
    cwd: projectRoot,
    stdio: ['inherit', 'ignore', 'inherit'],
});
if (result.error) throw result.error;
if (result.status !== 0) throw new Error(`yarn pack failed with status ${result.status}`);

const bytes = await readFile(archivePath);
const report = {
    schemaVersion: 1,
    package: metadata.name,
    version: metadata.version,
    filename: 'package.tgz',
    bytes: bytes.byteLength,
    sha256: createHash('sha256').update(bytes).digest('hex'),
};
await mkdir(dirname(reportPath), { recursive: true });
await writeFile(reportPath, `${JSON.stringify(report, null, 2)}\n`, 'utf8');
console.log(`Packed ${metadata.name}@${metadata.version} to ${archivePath}.`);
