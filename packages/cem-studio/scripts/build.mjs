import { spawnSync } from 'node:child_process';
import { mkdir, rm, writeFile } from 'node:fs/promises';
import { resolve } from 'node:path';
import { computeStudioDeploymentRevision, deploymentRevisionModule } from './deployment-revision.mjs';

const projectRoot = resolve(import.meta.dirname, '..');
const workspaceRoot = resolve(projectRoot, '../..');
const outputRoot = resolve(projectRoot, 'dist/static');
const reportPath = resolve(workspaceRoot, 'dist/reports/cem-studio/build.json');
const featureTourReportPath = resolve(workspaceRoot, 'dist/reports/cem-studio/feature-tour-build.json');
const cli = resolve(workspaceRoot, 'dist/target/cem_ml_cli/debug/cem-ml');
const generated = resolve(projectRoot, 'dist/deployment-inputs');

await rm(outputRoot, { recursive: true, force: true });
await rm(reportPath, { force: true });
await rm(featureTourReportPath, { force: true });
await mkdir(generated, { recursive: true });
await writeFile(resolve(generated, 'revision.js'), deploymentRevisionModule(''));

function emit(config, report) {
    const result = spawnSync(cli, [
        'transform',
        '--config',
        config,
        '--report-json',
        report,
    ], { cwd: workspaceRoot, stdio: 'inherit' });

    if (result.error) throw result.error;
    if (result.status !== 0) throw Error(`Native Studio graph failed: ${result.status}`);
}

emit('packages/cem-studio/generated/feature-tour/feature-tour.cem', 'dist/reports/cem-studio/feature-tour-build.json');
emit('packages/cem-studio/studio.cem', 'dist/reports/cem-studio/build.json');
// Hash the complete provisional deployment, including shell and sample bytes.
// Re-emit with a bounded metadata module; never patch graph-emitted JS/HTML.
const revision = await computeStudioDeploymentRevision(outputRoot);
await writeFile(resolve(generated, 'revision.js'), deploymentRevisionModule(revision.revision));
emit('packages/cem-studio/studio.cem', 'dist/reports/cem-studio/build.json');
await writeFile(resolve(workspaceRoot, 'dist/reports/cem-studio/deployment-revision.json'), `${JSON.stringify(revision, null, 2)}\n`);
