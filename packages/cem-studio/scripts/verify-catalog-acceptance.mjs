import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { createHash } from 'node:crypto';
import { mkdir, readFile, readdir, writeFile } from 'node:fs/promises';
import { resolve } from 'node:path';
import { catalogAcceptanceInputs, catalogAcceptanceStamp, catalogCensusVerifierIdentity, assertAcceptanceStamp, assertCatalogConfigurationEvidence, assertCatalogNavigationEvidence } from './catalog-acceptance.mjs';

const projectRoot = resolve(import.meta.dirname, '..'), workspaceRoot = resolve(projectRoot, '../..');
const reportRoot = resolve(workspaceRoot, 'dist/reports/cem-studio');
const json = async name => JSON.parse(await readFile(resolve(reportRoot, `${name}.json`), 'utf8'));
const inputs = await catalogAcceptanceInputs(workspaceRoot);
const stamp = await catalogAcceptanceStamp(workspaceRoot);
const stages = [
    ['catalog-configurations', 'verify-navigation.mjs', '--catalog-commands-only'],
    ['catalog-projects', 'verify-catalog-project-profiles.mjs'],
    ['populated-workspace', 'verify-populated-workspace.mjs'],
    ['navigation', 'verify-navigation.mjs'],
    ['static-worker', 'verify-static-worker.mjs'],
    ['presentation-samples', 'verify-static-worker.mjs', '--presentation-only'],
];
const execution = [];
if (!process.argv.includes('--check')) {
    for (const [report, script, ...args] of stages) {
        console.log(`[cem-studio:catalog-acceptance] ${report}`);
        await new Promise((resolveRun, reject) => {
            const child = spawn(process.execPath, [`scripts/${script}`, ...args], { cwd: projectRoot, stdio: 'inherit' });
            child.once('error', reject);
            child.once('exit', (code, signal) => code === 0 ? resolveRun() : reject(new Error(`${report} failed: ${signal ?? code}`)));
        });
        execution.push({ report, command: [script, ...args], exitCode: 0 });
    }
}
const [native, commands, navigation, domain, deployed, presentation, populated, deterministic] = await Promise.all([
    json('feature-tour'), json('catalog-configurations'), json('catalog-projects'), json('navigation'), json('static-worker'), json('presentation-samples'), json('populated-workspace'), json('determinism'),
]);
for (const [name, identity] of [['native', native.catalogIdentity], ['worker', commands.identity], ['navigation', navigation.identity]]) {
    assert.equal(identity, inputs.identity, `${name}: stale catalog acceptance evidence`);
}
const verifierIdentity = await catalogCensusVerifierIdentity(workspaceRoot);
assert.equal(commands.verifierIdentity, verifierIdentity, 'Worker configuration verifier changed');
assert.equal(navigation.verifierIdentity, verifierIdentity, 'Navigation census verifier changed');
const summaries = {
    native: assertCatalogConfigurationEvidence(inputs.projects, native.configurationReceipts, { host: 'native' }),
    workerOnline: assertCatalogConfigurationEvidence(inputs.projects, commands.evidence.receipts.online, { host: 'worker', phase: 'online' }),
    workerOffline: assertCatalogConfigurationEvidence(inputs.projects, commands.evidence.receipts.offline, { host: 'worker', phase: 'offline' }),
    navigation: assertCatalogNavigationEvidence(inputs.projects, navigation.evidence),
};
for (const phase of ['online', 'offline']) for (const project of inputs.projects) for (const configuration of project.configurations) {
    if (configuration.binary) continue;
    const receipt = commands.evidence.receipts[phase].find(row => row.project === project.id && row.configuration === configuration.id);
    assert.equal(receipt.operation, configuration.command ? 'configured-command' : 'validate-source',
        `${phase} ${project.id}/${configuration.id}: configured command execution and source validation are distinct`);
}
assert.equal(commands.evidence.offlineOutputsVerified, true);
assert.equal(commands.evidence.nativeOwnerIdentitiesAreNotPortable, true);
assert.equal(commands.evidence.originalsAndCopiesUnchanged, true);
for (const field of ['urlSelection', 'history', 'modelTrees', 'nestedCatalog', 'contextualAndLegacySampleUrls', 'collapsedCatalogSelection',
    'readonlyOriginals', 'independentClones', 'catalogDraftRetention', 'offlineCatalog', 'createCloneDelete', 'guardedDraft', 'themeRetention', 'bookmarkReload', 'supplierNativeRun', 'offline']) assert.equal(domain[field], true, field);
for (const field of ['markdownSite', 'dataProjects', 'csvInventory', 'xmlXhtmlProjects', 'htmlCemCard', 'svgMathmlProjects', 'stylesheetProjects',
    'dataQueryProjects', 'transformationProjects', 'schemaAuthoringProjects', 'moduleMapProject', 'portableConfigProject', 'nativeProjectionProject', 'catalogDocumentation']) assert.ok(domain[field] && Object.keys(domain[field]).length > 0, field);
assert.equal(domain.catalogDocumentation.nativeReadmeTables, true);
assert.equal(domain.catalogDocumentation.offlineReadmeTables, true);
for (const [label, report] of [['commands', commands], ['projects', navigation], ['navigation', domain], ['static-worker', deployed], ['presentation', presentation], ['populated-workspace', populated]]) assertAcceptanceStamp(report, stamp, label);
assert.equal(populated.evidence.catalogProjects, inputs.projects.length);
assert.equal(populated.evidence.copiesPerProject, 2);
assert.equal(populated.evidence.savedProjects, inputs.projects.length * 2);
assert.equal(populated.evidence.savedResources, inputs.projects.reduce((count, project) => count + project.resources.length * 2, 0));
for (const field of ['editorAttached', 'validationAttached', 'validationEnabled', 'originalsUnchanged', 'copiesUnchanged', 'noDraftsCreated']) assert.equal(populated.evidence[field], true, field);
assert.equal(deployed.source, 'graph-emitted-static-output');
assert.equal(deployed.featureTour.validatedCount, 32);
assert.equal(deployed.online.exitCode, 0); assert.equal(deployed.offline.exitCode, 0);
assert.equal(deployed.workbench.runtime, 'wasm-browser-worker');
assert.equal(presentation.loginConfigurations, 20);
for (const field of ['exampleProjects', 'validationRepair', 'templateOutput', 'offline', 'graphProvenance', 'liveView', 'sourceInspection', 'sourceFormatting', 'sourceColors', 'standaloneExpression']) assert.equal(presentation[field], true, field);
assert.equal(deterministic.cleanBuilds, 2);
const outputRoot = resolve(projectRoot, 'dist/static');
const files = [];
async function visit(path = '') {
    for (const item of await readdir(resolve(outputRoot, path), { withFileTypes: true })) {
        const relative = path ? `${path}/${item.name}` : item.name;
        if (item.isDirectory()) await visit(relative); else files.push(relative);
    }
}
await visit();
assert.deepEqual(files.sort(), deterministic.files.map(item => item.path).sort(), 'Deployed inventory differs from the verified clean build');
for (const item of deterministic.files) {
    const bytes = await readFile(resolve(outputRoot, item.path));
    assert.equal(bytes.length, item.bytes, item.path);
    assert.equal(createHash('sha256').update(bytes).digest('hex'), item.sha256, item.path);
}
assert.equal((await catalogAcceptanceInputs(workspaceRoot)).identity, inputs.identity, 'Catalog changed during acceptance');
const report = { schemaVersion: 1, catalogIdentity: inputs.identity, summaries, execution,
    scope: 'Declared configurations and current native/source-admission contracts; known limits are not clean conformance.',
    evidence: { native: 'feature-tour.json', worker: 'catalog-configurations.json', navigation: 'catalog-projects.json',
        buildsPreviewsRepairs: 'navigation.json', workerDelivery: 'static-worker.json', loginPresentation: 'presentation-samples.json', populatedWorkspace: 'populated-workspace.json', deterministic: 'determinism.json' },
    deployedSha256: deterministic.aggregateSha256, outputFiles: files.length,
    sharedDeclarativeControls: true, credentialPolicy: 'entered-passwords-and-one-time-codes-block-recovery', automaticWholeStudioRecovery: false };
await mkdir(reportRoot, { recursive: true });
await writeFile(resolve(reportRoot, 'catalog-acceptance.json'), `${JSON.stringify(report, null, 2)}\n`);
console.log(`Studio catalog acceptance: ${summaries.navigation.projects} projects, ${summaries.native.configurations} native and ${summaries.workerOnline.executed} online/offline worker executions, ${summaries.workerOnline.policyRefusals} explicit source-policy refusals.`);
