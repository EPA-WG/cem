import { createHash } from 'node:crypto';
import { spawnSync } from 'node:child_process';
import { mkdir, readFile, readdir, rm, writeFile } from 'node:fs/promises';
import { join, relative, resolve } from 'node:path';

const workspaceRoot = resolve(import.meta.dirname, '../../..');
const projectRoot = resolve(workspaceRoot, 'apps/cem-site');
const outputRoot = resolve(workspaceRoot, 'dist/apps/cem-site');
const reportRoot = resolve(workspaceRoot, 'dist/reports/cem-site');
const buildScript = resolve(projectRoot, 'scripts/build.mjs');
const manifest = JSON.parse(await readFile(resolve(projectRoot, 'site.routes.json'), 'utf8'));
const runtimeDestinationMap = JSON.parse(
    await readFile(resolve(workspaceRoot, manifest.runtime.destinationMap), 'utf8'),
);
const runtimeTargets = [
    ...Object.values(runtimeDestinationMap.imports),
    ...Object.values(runtimeDestinationMap.resources).map(({ path }) => path),
];
const runtimeOutputs = manifest.runtime.routes.flatMap((route) =>
    runtimeTargets.map((target) => new URL(target, `https://cem.invalid${route}`).pathname.slice(1)),
);

const expectedFiles = [
    ...manifest.entries.flatMap((entry) => [entry.output, `${entry.output}.map`]),
    ...runtimeOutputs,
    'site.report.json',
].sort();

async function filesUnder(directory) {
    const entries = await readdir(directory, { withFileTypes: true });
    const files = [];
    for (const entry of entries) {
        const path = join(directory, entry.name);
        if (entry.isDirectory()) {
            files.push(...(await filesUnder(path)));
        } else {
            files.push(relative(outputRoot, path).replaceAll('\\', '/'));
        }
    }
    return files.sort();
}

async function cleanBuildDigest(run) {
    await rm(outputRoot, { recursive: true, force: true });
    const result = spawnSync(process.execPath, [buildScript], {
        cwd: workspaceRoot,
        stdio: 'inherit',
    });
    if (result.error) {
        throw result.error;
    }
    if (result.status !== 0) {
        throw new Error(`clean CEM Site build ${run} failed with status ${result.status}`);
    }

    const actualFiles = await filesUnder(outputRoot);
    if (JSON.stringify(actualFiles) !== JSON.stringify(expectedFiles)) {
        throw new Error(
            `clean CEM Site build ${run} drifted.\nExpected: ${expectedFiles.join(', ')}\nActual: ${actualFiles.join(', ')}`,
        );
    }

    const files = [];
    for (const path of actualFiles) {
        const bytes = await readFile(join(outputRoot, path));
        files.push({
            path,
            bytes: bytes.byteLength,
            sha256: createHash('sha256').update(bytes).digest('hex'),
        });
    }
    return {
        files,
        aggregateSha256: createHash('sha256').update(JSON.stringify(files)).digest('hex'),
    };
}

const first = await cleanBuildDigest(1);
const second = await cleanBuildDigest(2);
if (JSON.stringify(first) !== JSON.stringify(second)) {
    const firstByPath = new Map(first.files.map((file) => [file.path, file]));
    const changed = second.files
        .filter((file) => firstByPath.get(file.path)?.sha256 !== file.sha256)
        .map((file) => file.path);
    throw new Error(`CEM Site clean builds are not deterministic: ${changed.join(', ') || 'aggregate drift'}`);
}

const report = {
    version: 1,
    cleanBuilds: 2,
    routeCount: manifest.entries.length,
    fileCount: second.files.length,
    aggregateSha256: second.aggregateSha256,
    files: second.files,
};
await mkdir(reportRoot, { recursive: true });
const reportPath = join(reportRoot, 'determinism.json');
await writeFile(reportPath, `${JSON.stringify(report, null, 2)}\n`, 'utf8');
console.log(`CEM Site determinism verified (${report.fileCount} files, sha256:${report.aggregateSha256})`);
