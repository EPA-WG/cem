import { spawnSync } from 'node:child_process';
import { rm } from 'node:fs/promises';
import { resolve } from 'node:path';

const projectRoot = resolve(import.meta.dirname, '..');
const workspaceRoot = resolve(projectRoot, '../..');
const outputRoot = resolve(projectRoot, 'dist/static');
const reportPath = resolve(workspaceRoot, 'dist/reports/cem-studio/build.json');
const featureTourReportPath = resolve(workspaceRoot, 'dist/reports/cem-studio/feature-tour-build.json');
const cli = resolve(workspaceRoot, 'dist/target/cem_ml_cli/debug/cem-ml');

await rm(outputRoot, { recursive: true, force: true });
await rm(reportPath, { force: true });
await rm(featureTourReportPath, { force: true });

for (const [config, report] of [
    ['packages/cem-studio/generated/feature-tour/feature-tour.cem', 'dist/reports/cem-studio/feature-tour-build.json'],
    ['packages/cem-studio/studio.cem', 'dist/reports/cem-studio/build.json'],
]) {
    const result = spawnSync(cli, [
        'transform',
        '--config',
        config,
        '--report-json',
        report,
    ], { cwd: workspaceRoot, stdio: 'inherit' });

    if (result.error) throw result.error;
    if (result.status !== 0) {
        process.exitCode = result.status ?? 1;
        break;
    }
}
