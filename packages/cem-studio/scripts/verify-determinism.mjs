import assert from 'node:assert/strict';
import { spawnSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { mkdir, readFile, readdir, writeFile } from 'node:fs/promises';
import { dirname, join, relative, resolve } from 'node:path';

const projectRoot = resolve(import.meta.dirname, '..');
const workspaceRoot = resolve(projectRoot, '../..');
const outputRoot = resolve(projectRoot, 'dist/static');
const buildScript = resolve(projectRoot, 'scripts/build.mjs');
const reportPath = resolve(workspaceRoot, 'dist/reports/cem-studio/determinism.json');

const first = await cleanBuildDigest(1);
const second = await cleanBuildDigest(2);
assert.deepEqual(second, first, 'two clean CEM Studio graph builds produced different bytes');

const report = {
    schemaVersion: 1,
    cleanBuilds: 2,
    fileCount: second.files.length,
    aggregateSha256: second.aggregateSha256,
    files: second.files,
};
await mkdir(dirname(reportPath), { recursive: true });
await writeFile(reportPath, `${JSON.stringify(report, null, 2)}\n`, 'utf8');
console.log(`CEM Studio determinism verified (${report.fileCount} files, sha256:${report.aggregateSha256}).`);

async function cleanBuildDigest(run) {
    const result = spawnSync(process.execPath, [buildScript], { cwd: projectRoot, stdio: 'inherit' });
    if (result.error) throw result.error;
    if (result.status !== 0) throw new Error(`clean CEM Studio build ${run} failed with status ${result.status}`);

    const files = [];
    for (const path of await filesUnder(outputRoot)) {
        const bytes = await readFile(resolve(outputRoot, path));
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

async function filesUnder(directory) {
    const entries = await readdir(directory, { withFileTypes: true });
    const files = [];
    for (const entry of entries) {
        const path = join(directory, entry.name);
        if (entry.isDirectory()) files.push(...(await filesUnder(path)));
        else files.push(relative(outputRoot, path).replaceAll('\\', '/'));
    }
    return files.sort();
}
