import { rm } from 'node:fs/promises';
import { spawnSync } from 'node:child_process';
import { resolve } from 'node:path';

const workspaceRoot = resolve(import.meta.dirname, '../../..');
const outputRoot = resolve(workspaceRoot, 'dist/apps/cem-site');
const cli = resolve(workspaceRoot, 'dist/target/cem_ml_cli/debug/cem-ml');

await rm(outputRoot, { recursive: true, force: true });

const result = spawnSync(
  cli,
  [
    'transform',
    '--config',
    'apps/cem-site/site.cem',
    '--report-json',
    'dist/apps/cem-site/site.report.json',
  ],
  { cwd: workspaceRoot, stdio: 'inherit' },
);

if (result.error) {
  throw result.error;
}
if (result.status !== 0) {
  process.exitCode = result.status ?? 1;
}
