import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { resolve } from 'node:path';
import { catalogAcceptanceInputs, catalogAcceptanceStamp, catalogCensusVerifierIdentity, assertAcceptanceStamp, assertCatalogNavigationEvidence } from './catalog-acceptance.mjs';

const projectRoot = resolve(import.meta.dirname, '..'), root = resolve(projectRoot, '../..');
const reportRoot = resolve(root, 'dist/reports/cem-studio');
const { projects, identity } = await catalogAcceptanceInputs(root), stamp = await catalogAcceptanceStamp(root);
const verifierIdentity = await catalogCensusVerifierIdentity(root);
const queue = [...projects], failures = [], reports = [];
const active = new Set();
const interrupt = () => { queue.length = 0; for (const child of active) child.kill('SIGTERM'); };
process.once('SIGTERM', interrupt); process.once('SIGINT', interrupt);
const readEvidence = async project => {
    const report = JSON.parse(await readFile(resolve(reportRoot, `catalog-projects-${project.id}.json`), 'utf8'));
    assertAcceptanceStamp(report, stamp, project.id);
    assert.equal(report.identity, identity); assert.equal(report.verifierIdentity, verifierIdentity);
    assertCatalogNavigationEvidence([project], report.evidence);
    return report;
};
async function worker() {
    while (queue.length) {
        const project = queue.shift();
        try {
            if (process.argv.includes('--reuse-current-profiles')) {
                const current = await readEvidence(project).catch(() => undefined);
                if (current) { reports.push(current); console.log(`[cem-studio:catalog-profiles] ${project.id}: current complete evidence reused`); continue; }
            }
            console.log(`[cem-studio:catalog-profiles] ${project.id}`);
            await new Promise((resolveRun, reject) => {
                const child = spawn(process.execPath, ['scripts/verify-navigation.mjs', '--catalog-projects-only', `--catalog-project=${project.id}`],
                    { cwd: projectRoot, stdio: 'inherit' });
                active.add(child);
                child.once('error', reject);
                child.once('exit', (code, signal) => {
                    active.delete(child);
                    if (code === 0) resolveRun();
                    else reject(new Error(`${project.id}: ${signal ?? code}`));
                });
            });
            reports.push(await readEvidence(project));
        } catch (error) { failures.push(error); }
    }
}
// Independent profiles preserve complete per-project UI/byte/draft assertions
// without making another project's retained UI part of its acceptance fixture.
await Promise.allSettled([worker(), worker(), worker()]).then(results => {
    for (const result of results) if (result.status === 'rejected') failures.push(result.reason);
});
process.removeListener('SIGTERM', interrupt); process.removeListener('SIGINT', interrupt);
if (failures.length) throw new AggregateError(failures, 'Catalog project profiles failed');
const evidence = { coverage: 'navigation-clone-admission-offline',
    projects: reports.flatMap(report => report.evidence.projects),
    unchangedOriginalProjectIds: reports.flatMap(report => report.evidence.unchangedOriginalProjectIds),
    executedConfigurationIds: [], isolatedProfiles: reports.length,
    draft: { persistedBytesUnchanged: reports.every(report => report.evidence.draft.persistedBytesUnchanged) },
    drafts: reports.map(report => report.evidence.draft) };
assertCatalogNavigationEvidence(projects, evidence);
assert.deepEqual(await catalogAcceptanceStamp(root), stamp, 'Deployment changed during the catalog census');
assert.equal(await catalogCensusVerifierIdentity(root), verifierIdentity, 'Census verifier changed during acceptance');
await mkdir(reportRoot, { recursive: true });
await writeFile(resolve(reportRoot, 'catalog-projects.json'), `${JSON.stringify({ schemaVersion: 1, ...stamp, identity, verifierIdentity, evidence }, null, 2)}\n`);
console.log(`Catalog navigation: ${projects.length} complete isolated project profiles passed.`);
