import { spawnSync } from 'node:child_process';
import { mkdirSync, readFileSync, rmSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const projectRoot = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const workspaceRoot = resolve(projectRoot, '../..');
const packageMetadata = JSON.parse(readFileSync(resolve(projectRoot, 'package.json'), 'utf8'));
const outputRoot = resolve(workspaceRoot, 'dist/packages/cem-ml-npm');
const archivePath = resolve(outputRoot, 'package.tgz');

mkdirSync(outputRoot, { recursive: true });
rmSync(archivePath, { force: true });
const result = spawnSync(process.platform === 'win32' ? 'yarn.cmd' : 'yarn', [
  'pack',
  '--out',
  archivePath,
], {
  cwd: projectRoot,
  stdio: ['inherit', 'ignore', 'inherit'],
});
if (result.status !== 0) {
  throw new Error(`yarn pack failed with status ${result.status}`);
}

console.log(`Packed ${packageMetadata.name}@${packageMetadata.version} to ${archivePath}`);
