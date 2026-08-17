import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { mkdirSync, mkdtempSync, readFileSync, rmSync, statSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { resolve } from 'node:path';
import test from 'node:test';

import { attestNpmReleaseEvidence } from './cem-ml-npm-release-evidence.mjs';
import {
    expectedReleaseUnits,
    stagePlatformRelease,
    uploadPlatformReleaseDraft,
    verifyPlatformRelease,
} from './cem-ml-platform-release.mjs';

const version = '3.4.5';
const sourceCommit = '1234567890abcdef1234567890abcdef12345678';

test('five deployment fixtures stage one complete immutable release index', () => {
    const fixture = createFixture();
    try {
        const staged = stagePlatformRelease({
            workspaceRoot: fixture.root,
            version,
            sourceCommit,
            units: expectedReleaseUnits,
        });
        assert.equal(staged.index.units.length, 5);
        assert.equal(staged.index.releaseTag, `cem-ml-v${version}`);
        assert.ok(staged.index.assets.length > 25);
        assert.equal(
            verifyPlatformRelease({
                workspaceRoot: fixture.root,
                outputRoot: staged.outputRoot,
                version,
                sourceCommit,
                units: expectedReleaseUnits,
            }).sourceCommit,
            sourceCommit,
        );
    } finally {
        fixture.dispose();
    }
});

test('GitHub draft upload remains inert without the protected release opt-in', () => {
    assert.throws(() => uploadPlatformReleaseDraft(), /draft upload is disabled/);
});

test('required npm signing rejects an absent GitHub attestation bundle', () => {
    const fixture = createFixture();
    const previousSigning = process.env.CEM_ML_RELEASE_SIGNING;
    const previousBundle = process.env.CEM_ML_GITHUB_ATTESTATION_BUNDLE;
    try {
        process.env.CEM_ML_RELEASE_SIGNING = 'required';
        delete process.env.CEM_ML_GITHUB_ATTESTATION_BUNDLE;
        assert.throws(
            () => attestNpmReleaseEvidence({ workspaceRoot: fixture.root, packageName: '@epa-wg/cem-ml' }),
            /requires CEM_ML_GITHUB_ATTESTATION_BUNDLE/,
        );
    } finally {
        restoreEnvironment('CEM_ML_RELEASE_SIGNING', previousSigning);
        restoreEnvironment('CEM_ML_GITHUB_ATTESTATION_BUNDLE', previousBundle);
        fixture.dispose();
    }
});

const driftCases = [
    ['version', ({ entry }) => updateJson(entry, (value) => (value.commonVersion = '9.9.9'))],
    ['source commit', ({ entry }) => updateJson(entry, (value) => (value.sourceCommit = 'f'.repeat(40)))],
    ['target', ({ entry }) => updateJson(entry, (value) => (value.targetIdentities = ['wrong-target']))],
    ['checksum', ({ primary }) => writeFileSync(primary, 'changed bytes')],
    ['signature', ({ signing }) => updateJson(signing, (value) => (value.checksumManifest.sha256 = '0'.repeat(64)))],
    ['SBOM', ({ sbom }) => updateJson(sbom, (value) => (value.spdxVersion = 'SPDX-2.2'))],
    [
        'provenance',
        ({ provenance }) =>
            updateJson(
                provenance,
                (value) => (value.buildDefinition.resolvedDependencies[0].digest.gitCommit = 'a'.repeat(40)),
            ),
    ],
    ['capability', ({ capability }) => updateJson(capability, (value) => (value.commonVersion = '0.0.0'))],
    ['release index', ({ entry }) => updateJson(entry, (value) => (value.capabilityManifestDigest = '1'.repeat(64)))],
];

for (const [label, mutate] of driftCases) {
    test(`platform staging rejects independent ${label} drift`, () => {
        const fixture = createFixture();
        try {
            mutate(fixture.npmRuntime);
            assert.throws(
                () =>
                    stagePlatformRelease({
                        workspaceRoot: fixture.root,
                        version,
                        sourceCommit,
                        units: expectedReleaseUnits,
                    }),
                /drift|expected|missing|version|target|source|checksum|SBOM|provenance|capability/i,
            );
        } finally {
            fixture.dispose();
        }
    });
}

function createFixture() {
    const root = mkdtempSync(resolve(tmpdir(), 'cem-ml-platform-release-'));
    let npmRuntime;
    for (const unit of expectedReleaseUnits) {
        const created = writeUnit(root, unit);
        if (unit.identity === '@epa-wg/cem-ml') npmRuntime = created;
    }
    return { root, npmRuntime, dispose: () => rmSync(root, { recursive: true, force: true }) };
}

function writeUnit(workspaceRoot, unit) {
    const root = resolve(workspaceRoot, unit.root);
    mkdirSync(root, { recursive: true });
    const coordinate = unit.identity.replace('@epa-wg/', '').replaceAll(/[^a-z0-9]+/g, '-');
    const base = `cem-ml-${version}-${coordinate}`;
    const primaryName = `${base}.${unit.identity.startsWith('native-') ? 'zip' : 'tgz'}`;
    const capabilityName = `${base}.capabilities.json`;
    const integrityName = `${base}.integrity.json`;
    const sbomName = `${base}.spdx.json`;
    const provenanceName = `${base}.provenance.json`;
    const channelName = unit.channel ? `${base}.${unit.channel}.json` : null;
    const entryName = `${base}.release-index-entry.json`;
    const checksumName = `${base}.sha256`;
    const signingName = `${base}.signing.json`;
    const primary = resolve(root, primaryName);
    const capability = resolve(root, capabilityName);
    const integrity = resolve(root, integrityName);
    const sbom = resolve(root, sbomName);
    const provenance = resolve(root, provenanceName);
    const entry = resolve(root, entryName);
    const signing = resolve(root, signingName);
    writeFileSync(primary, `${unit.identity} package bytes`);
    writeJson(capability, {
        commonVersion: version,
        runtime: unit.identity,
        targetIdentity: unit.target ?? unit.targets[0],
    });
    if (unit.identity.startsWith('@')) writeJson(integrity, { commonVersion: version, algorithm: 'sha256', files: [] });
    writeJson(sbom, {
        spdxVersion: 'SPDX-2.3',
        dataLicense: 'CC0-1.0',
        name: `${unit.identity}-${version}`,
        packages: [{ name: unit.identity, versionInfo: version }],
    });
    const provenanceSubjects = [primaryName, capabilityName, sbomName];
    if (unit.identity.startsWith('@')) provenanceSubjects.push(integrityName);
    writeJson(provenance, {
        predicateType: 'https://slsa.dev/provenance/v1',
        buildDefinition: {
            resolvedDependencies: [
                { uri: 'git+https://github.com/EPA-WG/cem.git', digest: { gitCommit: sourceCommit } },
            ],
        },
        subject: provenanceSubjects.map((filename) => artifactRecord(root, filename)),
    });

    const artifactNames = [primaryName, capabilityName, sbomName, provenanceName];
    if (unit.identity.startsWith('@')) artifactNames.splice(2, 0, integrityName);
    if (channelName) {
        writeJson(resolve(root, channelName), {
            channel: unit.channel,
            version,
            immutableSource: {
                releaseTag: `cem-ml-v${version}`,
                filename: primaryName,
                url: `https://github.com/EPA-WG/cem/releases/download/cem-ml-v${version}/${primaryName}`,
                sha256: sha256File(primary),
            },
        });
        artifactNames.push(channelName);
    }
    const releaseEntry = {
        schemaVersion: 1,
        product: 'cem-ml',
        commonVersion: version,
        sourceCommit,
        releaseTag: `cem-ml-v${version}`,
        capabilityManifestDigest: sha256File(capability),
        artifacts: artifactNames.map((filename) => artifactRecord(root, filename)),
        checksumManifest: checksumName,
        signingRecord: signingName,
        publicationState: 'staged-local',
    };
    if (unit.identity.startsWith('@')) {
        releaseEntry.npmIdentity = unit.identity;
        releaseEntry.runtimeIdentities = ['wasm-browser-worker', 'wasm-node'];
        releaseEntry.targetIdentities = [...unit.targets];
        releaseEntry.abiIdentities = ['fixture-abi'];
        releaseEntry.integrityManifestDigest = sha256File(integrity);
    } else {
        releaseEntry.runtimeIdentity = unit.identity;
        releaseEntry.targetIdentity = unit.target;
        releaseEntry.abiIdentity = 'fixture-abi';
    }
    writeJson(entry, releaseEntry);
    const checksumFiles = [...artifactNames, entryName].sort();
    writeFileSync(
        resolve(root, checksumName),
        `${checksumFiles.map((filename) => `${sha256File(resolve(root, filename))}  ${filename}`).join('\n')}\n`,
    );
    writeJson(signing, {
        commonVersion: version,
        releaseTag: `cem-ml-v${version}`,
        checksumManifest: { filename: checksumName, sha256: sha256File(resolve(root, checksumName)) },
        githubArtifactAttestation: { status: 'awaiting-github-oidc', bundle: null, sha256: null },
        publicationReady: false,
    });
    return { root, primary, capability, integrity, sbom, provenance, entry, signing };
}

function artifactRecord(root, filename) {
    const path = resolve(root, filename);
    return { filename, byteLength: statSync(path).size, sha256: sha256File(path) };
}

function sha256File(path) {
    return createHash('sha256').update(readFileSync(path)).digest('hex');
}

function writeJson(path, value) {
    writeFileSync(path, `${JSON.stringify(value, null, 2)}\n`);
}

function updateJson(path, mutate) {
    const value = JSON.parse(readFileSync(path, 'utf8'));
    mutate(value);
    writeJson(path, value);
}

function restoreEnvironment(name, value) {
    if (value === undefined) delete process.env[name];
    else process.env[name] = value;
}
