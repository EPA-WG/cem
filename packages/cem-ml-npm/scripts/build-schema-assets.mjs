import { spawnSync } from 'node:child_process';
import { cpSync, existsSync, mkdtempSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, join, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const projectRoot = resolve(dirname(fileURLToPath(import.meta.url)), '..');
const workspaceRoot = resolve(projectRoot, '../..');
const arguments_ = process.argv.slice(2);
if (arguments_.length !== 0 && (arguments_.length !== 2 || arguments_[0] !== '--out')) {
  throw new Error('usage: node build-schema-assets.mjs [--out <directory>]');
}
const outputRoot = arguments_.length ? resolve(arguments_[1]) : resolve(projectRoot, 'dist-schema-assets');
if (arguments_.length && existsSync(outputRoot)) {
  throw new Error('Explicit schema-asset output must not already exist');
}
const stagingRoot = mkdtempSync(join(tmpdir(), 'cem-schema-assets-build-'));
try {
  const result = spawnSync('cargo', [
    'run',
    '--locked',
    '--package',
    'cem-ml',
    '--no-default-features',
    '--bin',
    'cem-ml-schema-assets-emit',
    '--target-dir',
    resolve(workspaceRoot, 'dist/target/cem_ml_schema_assets'),
    '--',
    '--out',
    stagingRoot,
  ], { cwd: workspaceRoot, stdio: 'inherit' });
  if (result.status !== 0) {
    throw new Error(`Native schema-asset emission failed: ${result.error?.message ?? result.status}`);
  }
  if (!arguments_.length) rmSync(outputRoot, { recursive: true, force: true });
  cpSync(stagingRoot, outputRoot, { recursive: true, force: false, errorOnExist: true });
  console.log(`Built optional native schema assets in ${outputRoot}`);
} finally {
  rmSync(stagingRoot, { recursive: true, force: true });
}
