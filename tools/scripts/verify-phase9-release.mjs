#!/usr/bin/env node

import { spawnSync } from 'node:child_process';
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const workspaceRoot = resolve(dirname(fileURLToPath(import.meta.url)), '../..');
const defaultContractPath = 'tools/fitness/phase9-release-contract.json';
const reportPath = resolve(workspaceRoot, 'dist/reports/phase9-release-readiness.json');
const markdownReportPath = resolve(workspaceRoot, 'dist/reports/phase9-release-readiness.md');

export async function gatherWorkspaceState(contract, root = workspaceRoot) {
    const manifestPaths = new Set();
    for (const family of contract.versionFamilies) {
        if (family.authorityType === 'npm') manifestPaths.add(family.authority);
        for (const member of family.members) manifestPaths.add(member.manifest);
    }
    for (const dependency of contract.exactDependencies) manifestPaths.add(dependency.manifest);
    for (const packageContract of contract.publicPackages) manifestPaths.add(packageContract.manifest);

    const manifests = {};
    for (const path of manifestPaths) manifests[path] = await readJson(resolve(root, path));

    const familyVersions = {};
    for (const family of contract.versionFamilies) {
        const authorityVersion =
            family.authorityType === 'cargo'
                ? cargoVersion(await readFile(resolve(root, family.authority), 'utf8'))
                : manifests[family.authority]?.version;
        familyVersions[family.id] = { authority: authorityVersion };
        for (const member of family.members) {
            familyVersions[family.id][member.manifest] = manifests[member.manifest]?.version;
        }
    }

    const projects = new Set([
        '@epa-wg/cem',
        ...contract.publicPackages.map((packageContract) => packageContract.project),
    ]);
    const nxTargets = {};
    for (const project of projects) nxTargets[project] = await resolvedNxTargets(project, root);

    const packageExports = {};
    const packageBins = {};
    for (const packageContract of contract.publicPackages) {
        const manifest = manifests[packageContract.manifest];
        packageExports[packageContract.manifest] = Object.keys(manifest.exports ?? {});
        packageBins[packageContract.manifest] = Object.keys(manifest.bin ?? {});
    }

    const requiredPaths = new Set([
        ...contract.requiredDocuments,
        ...contract.deprecationSources,
        ...contract.workflows.map((workflow) => workflow.path),
    ]);
    const existingPaths = [];
    for (const path of requiredPaths) {
        if (await canRead(resolve(root, path))) existingPaths.push(path);
    }

    const workflowTexts = {};
    for (const workflow of contract.workflows) {
        workflowTexts[workflow.path] = await readOptionalText(resolve(root, workflow.path));
    }

    const policyText = await readOptionalText(resolve(root, 'docs/versioning-and-compatibility.md'));
    const publicationEvidence = await readOptionalJson(resolve(root, contract.publicationEvidence.path));

    return {
        manifests,
        nx: await readJson(resolve(root, 'nx.json')),
        nxTargets,
        familyVersions,
        packageExports,
        packageBins,
        existingPaths,
        workflowTexts,
        policyText,
        publicationEvidence,
        revision: git(['rev-parse', 'HEAD'], root),
    };
}

export function validatePhase9Contract(contract, state, mode = 'readiness') {
    const errors = [];
    const blockers = [];
    if (contract.schemaVersion !== 1) errors.push('phase9 contract schemaVersion must be 1');

    const familyAuthority = {};
    for (const family of contract.versionFamilies) {
        const versions = state.familyVersions[family.id] ?? {};
        const authority = versions.authority;
        familyAuthority[family.id] = authority;
        if (!isSemver(authority)) errors.push(`version family ${family.id} has invalid authority ${authority}`);
        for (const member of family.members) {
            const actual = versions[member.manifest];
            if (actual !== authority) {
                errors.push(
                    `version family ${family.id} drift: ${member.manifest} is ${actual ?? 'missing'}, expected ${authority}`,
                );
            }
        }

        const group = state.nx.release?.groups?.[family.releaseGroup];
        if (!group) {
            errors.push(`missing Nx release group ${family.releaseGroup}`);
            continue;
        }
        if (group.projectsRelationship !== 'fixed') {
            errors.push(`Nx release group ${family.releaseGroup} must be fixed`);
        }
        const expectedProjects = family.releaseGroupProjects ?? family.members.map((member) => member.project);
        if (!sameSet(group.projects, expectedProjects)) {
            errors.push(`Nx release group ${family.releaseGroup} has the wrong project membership`);
        }
    }

    for (const dependency of contract.exactDependencies) {
        const manifest = state.manifests[dependency.manifest];
        const field = dependency.field ?? 'dependencies';
        const actual = manifest?.[field]?.[dependency.dependency];
        const expected = familyAuthority[dependency.family];
        if (actual !== expected) {
            errors.push(
                `${dependency.manifest} ${field} entry ${dependency.dependency} must be exact ${expected}, found ${actual ?? 'missing'}`,
            );
        }
    }

    for (const packageContract of contract.publicPackages) {
        const manifest = state.manifests[packageContract.manifest];
        if (!manifest) {
            errors.push(`missing public package manifest ${packageContract.manifest}`);
            continue;
        }
        if (manifest.private === true) errors.push(`${manifest.name} must not be private`);
        if (!manifest.license) errors.push(`${manifest.name} is missing a license`);
        if (manifest.publishConfig?.access !== 'public') errors.push(`${manifest.name} must publish with public access`);
        if (!Array.isArray(manifest.files) || manifest.files.length === 0) {
            errors.push(`${manifest.name} is missing a bounded files list`);
        }
        if (!String(manifest.repository?.url ?? '').includes('EPA-WG/cem')) {
            errors.push(`${manifest.name} repository must identify the CEM monorepo`);
        }
        const exports = state.packageExports[packageContract.manifest] ?? [];
        for (const requiredExport of packageContract.requiredExports) {
            if (!exports.includes(requiredExport)) {
                errors.push(`${manifest.name} missing stable export ${requiredExport}`);
            }
        }
        const bins = state.packageBins[packageContract.manifest] ?? [];
        for (const requiredBin of packageContract.requiredBins ?? []) {
            if (!bins.includes(requiredBin)) errors.push(`${manifest.name} missing executable ${requiredBin}`);
        }
        const targets = state.nxTargets[packageContract.project] ?? [];
        if (!targets.includes(packageContract.verificationTarget)) {
            errors.push(`${packageContract.project} missing verification target ${packageContract.verificationTarget}`);
        }
    }

    for (const path of contract.requiredDocuments) {
        if (!state.existingPaths.includes(path)) errors.push(`missing required Phase 9 document ${path}`);
    }
    for (const axis of contract.contractAxes) {
        if (!state.policyText.includes(axis.policyText)) {
            errors.push(`compatibility axis ${axis.id} is missing policy ownership text ${axis.policyText}`);
        }
    }
    for (const path of contract.deprecationSources) {
        if (!state.existingPaths.includes(path)) errors.push(`missing deprecation source ${path}`);
    }
    for (const workflow of contract.workflows) {
        const text = state.workflowTexts[workflow.path] ?? '';
        for (const requiredText of workflow.requiredText) {
            if (!text.includes(requiredText)) {
                errors.push(`missing required workflow contract ${requiredText} in ${workflow.path}`);
            }
        }
    }

    const rootTargets = state.nxTargets['@epa-wg/cem'] ?? [];
    for (const target of Object.values(contract.aggregateTargets)) {
        if (!rootTargets.includes(target)) errors.push(`root project missing aggregate target ${target}`);
    }

    if (mode === 'release') {
        validatePublicationEvidence(contract, state, familyAuthority, errors);
    } else if (!state.publicationEvidence) {
        blockers.push(`public release evidence pending at ${contract.publicationEvidence.path}`);
    } else {
        const evidenceErrors = [];
        validatePublicationEvidence(contract, state, familyAuthority, evidenceErrors);
        if (evidenceErrors.length > 0) blockers.push(...evidenceErrors);
    }

    return { errors, blockers, familyAuthority };
}

function validatePublicationEvidence(contract, state, familyAuthority, errors) {
    const evidence = state.publicationEvidence;
    if (!evidence) {
        errors.push(`publication evidence missing at ${contract.publicationEvidence.path}`);
        return;
    }
    if (evidence.schemaVersion !== 1) errors.push('publication evidence schemaVersion must be 1');
    if (evidence.status !== contract.publicationEvidence.requiredStatus) {
        errors.push(`publication evidence status must be ${contract.publicationEvidence.requiredStatus}`);
    }
    for (const path of contract.publicationEvidence.requiredPaths) {
        if (!valueAtPath(evidence, path)) errors.push(`publication evidence missing ${path}`);
    }
    if (evidence.web?.version !== familyAuthority['cem-web']) {
        errors.push(`publication evidence web.version must be ${familyAuthority['cem-web']}`);
    }
    if (evidence.cemMl?.version !== familyAuthority['cem-ml-platform']) {
        errors.push(`publication evidence cemMl.version must be ${familyAuthority['cem-ml-platform']}`);
    }
    if (evidence.studio?.version !== familyAuthority['cem-ml-platform']) {
        errors.push(`publication evidence studio.version must be ${familyAuthority['cem-ml-platform']}`);
    }
    if (!/^[0-9a-f]{40}$/.test(evidence.source?.revision ?? '')) {
        errors.push('publication evidence source.revision must be a full Git commit');
    }
    for (const url of collectEvidenceUrls(evidence)) {
        if (!url.startsWith('https://')) errors.push(`publication evidence URL must use HTTPS: ${url}`);
    }
}

function collectEvidenceUrls(evidence) {
    const urls = [];
    const visit = (value, key = '') => {
        if (typeof value === 'string' && /(?:url|Url)$/.test(key)) urls.push(value);
        if (Array.isArray(value)) value.forEach((item) => visit(item));
        else if (value && typeof value === 'object') {
            for (const [childKey, child] of Object.entries(value)) visit(child, childKey);
        }
    };
    visit(evidence);
    return urls;
}

function valueAtPath(value, path) {
    return path.split('.').reduce((current, segment) => current?.[segment], value);
}

function cargoVersion(source) {
    return /^version\s*=\s*"([^"]+)"/m.exec(source)?.[1];
}

function isSemver(value) {
    return typeof value === 'string' && /^\d+\.\d+\.\d+(?:-[0-9A-Za-z.-]+)?(?:\+[0-9A-Za-z.-]+)?$/.test(value);
}

function sameSet(left = [], right = []) {
    return left.length === right.length && left.every((value) => right.includes(value));
}

async function resolvedNxTargets(project, root) {
    const executable = process.platform === 'win32' ? 'yarn.cmd' : 'yarn';
    const result = spawnSync(executable, ['nx', 'show', 'project', project, '--json'], {
        cwd: root,
        encoding: 'utf8',
        env: { ...process.env, NX_TUI: 'false' },
    });
    if (result.status !== 0) {
        throw new Error(`nx show project ${project} failed: ${result.stderr || result.stdout || result.error}`);
    }
    return Object.keys(JSON.parse(result.stdout).targets ?? {});
}

function git(args, root) {
    const result = spawnSync('git', args, { cwd: root, encoding: 'utf8' });
    if (result.status !== 0) throw new Error(`git ${args.join(' ')} failed: ${result.stderr || result.error}`);
    return result.stdout.trim();
}

async function canRead(path) {
    try {
        await readFile(path);
        return true;
    } catch {
        return false;
    }
}

async function readJson(path) {
    return JSON.parse(await readFile(path, 'utf8'));
}

async function readOptionalJson(path) {
    try {
        return await readJson(path);
    } catch (error) {
        if (error?.code === 'ENOENT') return null;
        throw error;
    }
}

async function readOptionalText(path) {
    try {
        return await readFile(path, 'utf8');
    } catch (error) {
        if (error?.code === 'ENOENT') return '';
        throw error;
    }
}

async function writeReport(contract, state, mode, result) {
    const report = {
        schemaVersion: 1,
        mode,
        sourceRevision: state.revision,
        ready: result.errors.length === 0,
        closureReady: result.errors.length === 0 && result.blockers.length === 0,
        familyVersions: result.familyAuthority,
        publicPackages: contract.publicPackages.map((packageContract) => ({
            name: state.manifests[packageContract.manifest]?.name,
            version: state.manifests[packageContract.manifest]?.version,
            exports: state.packageExports[packageContract.manifest]?.length ?? 0,
            verificationTarget: `${packageContract.project}:${packageContract.verificationTarget}`,
        })),
        compatibilityAxes: contract.contractAxes.map((axis) => ({ id: axis.id, owner: axis.owner })),
        errors: result.errors,
        blockers: result.blockers,
    };
    const markdown = [
        '# Phase 9 release readiness',
        '',
        `- Mode: ${mode}`,
        `- Source revision: \`${state.revision}\``,
        `- Credential-free readiness: ${report.ready ? 'pass' : 'fail'}`,
        `- Public closure: ${report.closureReady ? 'ready' : 'pending'}`,
        `- Public packages checked: ${report.publicPackages.length}`,
        `- Compatibility axes checked: ${report.compatibilityAxes.length}`,
        '',
        '## Blockers',
        '',
        ...(report.blockers.length > 0 ? report.blockers.map((blocker) => `- ${blocker}`) : ['- None.']),
        '',
        '## Errors',
        '',
        ...(report.errors.length > 0 ? report.errors.map((error) => `- ${error}`) : ['- None.']),
        '',
    ].join('\n');
    await mkdir(dirname(reportPath), { recursive: true });
    await writeFile(reportPath, `${JSON.stringify(report, null, 2)}\n`, 'utf8');
    await writeFile(markdownReportPath, markdown, 'utf8');
}

async function main() {
    const modeIndex = process.argv.indexOf('--mode');
    const mode = modeIndex >= 0 ? process.argv[modeIndex + 1] : 'readiness';
    if (!['readiness', 'release'].includes(mode)) throw new Error(`unsupported mode ${mode}`);
    const contract = await readJson(resolve(workspaceRoot, defaultContractPath));
    const state = await gatherWorkspaceState(contract);
    const result = validatePhase9Contract(contract, state, mode);
    await writeReport(contract, state, mode, result);
    if (result.errors.length > 0) {
        for (const error of result.errors) console.error(`phase9: ${error}`);
        process.exitCode = 1;
        return;
    }
    console.log(
        `Phase 9 ${mode} contract verified (${contract.publicPackages.length} public packages, ` +
            `${contract.contractAxes.length} compatibility axes, ${result.blockers.length} closure blocker).`,
    );
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
    main().catch((error) => {
        console.error(error);
        process.exit(1);
    });
}
