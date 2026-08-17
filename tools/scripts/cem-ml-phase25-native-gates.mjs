import { spawnSync } from 'node:child_process';
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const workspaceRoot = resolve(dirname(fileURLToPath(import.meta.url)), '../..');
const deployments = [
    'packages/cem-ml-cli-native-linux-amd64',
    'packages/cem-ml-cli-native-brew-arm64',
    'packages/cem-ml-cli-native-windows-amd64',
].map((projectRoot) => ({
    projectRoot,
    deployment: JSON.parse(readFileSync(resolve(workspaceRoot, projectRoot, 'deployment.json'), 'utf8')),
}));
const targets = ['verify', 'smoke:install', 'smoke:upgrade', 'smoke:uninstall'];
const report = {
    schemaVersion: 1,
    host: { platform: process.platform, architecture: process.arch },
    deployments: [],
};
const reportPath = resolve(workspaceRoot, 'dist/reports/cem-ml-phase25-native-gates.json');

try {
    for (const { deployment } of deployments) {
        if (
            deployment.host.platform !== process.platform ||
            deployment.host.architecture !== process.arch
        ) {
            report.deployments.push({
                project: deployment.nxProject,
                runtimeIdentity: deployment.runtimeIdentity,
                rustTarget: deployment.rustTarget,
                status: 'unavailable',
                requiredHost: deployment.host,
                reason:
                    `requires ${deployment.host.platform}/${deployment.host.architecture}; ` +
                    `current host is ${process.platform}/${process.arch}`,
                targets: targets.map((target) => ({ target, status: 'unavailable' })),
            });
            continue;
        }

        const entry = {
            project: deployment.nxProject,
            runtimeIdentity: deployment.runtimeIdentity,
            rustTarget: deployment.rustTarget,
            status: 'running',
            requiredHost: deployment.host,
            targets: [],
        };
        report.deployments.push(entry);
        for (const target of targets) {
            const result = runNxTarget(`${deployment.nxProject}:${target}`);
            entry.targets.push({
                target,
                status: result.status === 0 ? 'passed' : 'failed',
                exitCode: result.status,
            });
            if (result.status !== 0) {
                entry.status = 'failed';
                throw new Error(`${deployment.nxProject}:${target} failed with exit ${result.status}`);
            }
        }
        entry.status = 'passed';
    }
} finally {
    mkdirSync(dirname(reportPath), { recursive: true });
    writeFileSync(reportPath, `${JSON.stringify(report, null, 2)}\n`);
}

const passed = report.deployments.filter(({ status }) => status === 'passed');
const unavailable = report.deployments.filter(({ status }) => status === 'unavailable');
if (passed.length !== 1 || unavailable.length !== 2) {
    throw new Error(
        `expected one available native deployment and two unavailable deployments; ` +
            `observed ${passed.length} passed and ${unavailable.length} unavailable`,
    );
}
console.log(
    `Verified ${passed[0].runtimeIdentity} native lifecycle; explicitly unavailable: ` +
        unavailable.map(({ runtimeIdentity }) => runtimeIdentity).join(', '),
);

function runNxTarget(target) {
    const executable = process.platform === 'win32' ? 'yarn.cmd' : 'yarn';
    return spawnSync(executable, ['nx', 'run', target, '--parallel=false'], {
        cwd: workspaceRoot,
        env: { ...process.env, NX_DAEMON: 'false' },
        encoding: 'utf8',
        stdio: 'inherit',
    });
}
