import { spawnSync } from 'node:child_process';
import { mkdirSync, readFileSync, rmSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

import { emitNpmReleaseEvidence } from '../../../tools/scripts/cem-ml-npm-release-evidence.mjs';
import { expectedReleaseUnits, validateReleaseUnit } from '../../../tools/scripts/cem-ml-platform-release.mjs';

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

const releaseEvidence = emitNpmReleaseEvidence({
  workspaceRoot,
  projectRoot,
  outputRoot,
  archivePath,
  coordinate: 'wasm-runtime-npm',
  runtimeManifestPath: resolve(projectRoot, 'dist/cem-ml-runtime.json'),
  integrityManifestPath: resolve(projectRoot, 'dist/integrity.json'),
});
validateReleaseUnit({
  root: releaseEvidence.artifactRoot,
  unit: expectedReleaseUnits.find(({ identity }) => identity === packageMetadata.name),
  version: releaseEvidence.version,
  sourceCommit: releaseEvidence.sourceCommit,
  releaseTag: releaseEvidence.releaseTag,
});

console.log(`Packed ${packageMetadata.name}@${packageMetadata.version} to ${archivePath}`);
