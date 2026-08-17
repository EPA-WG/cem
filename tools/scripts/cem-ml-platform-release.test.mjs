import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { mkdirSync, mkdtempSync, readFileSync, rmSync, statSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import test from 'node:test';

import { attestNpmReleaseEvidence } from './cem-ml-npm-release-evidence.mjs';
import {
    createOrResumePlatformReleaseDraft,
    expectedReleaseUnits,
    githubDraftCreateArguments,
    stagePlatformRelease,
    uploadPlatformReleaseDraft,
    verifyPlatformRelease,
} from './cem-ml-platform-release.mjs';

const workspaceRoot = resolve(dirname(fileURLToPath(import.meta.url)), '../..');
const version = '3.4.5';
const sourceCommit = '1234567890abcdef1234567890abcdef12345678';
const releaseTag = `cem-ml-v${version}`;
const releaseTitle = `CEM-ML ${version}`;

test('absent GitHub release creates one notes-bounded draft without a publish path', () => {
    const github = createFakeGithubReleaseClient();
    const result = createOrResumePlatformReleaseDraft({
        authorized: true,
        version,
        sourceCommit,
        releaseTag,
        taggedCommit: sourceCommit,
        previousReleaseTag: 'cem-ml-v3.4.4',
        github,
    });

    assert.equal(result.action, 'created');
    assert.deepEqual(github.createRequests, [
        {
            tag: releaseTag,
            title: releaseTitle,
            draft: true,
            prerelease: false,
            generateNotes: true,
            notesStartTag: 'cem-ml-v3.4.4',
        },
    ]);
    assert.equal(github.publishCalls, 0, 'draft coordinator must not expose publication');
});

test('GitHub CLI draft creation uses the existing tag and bounded generated notes', () => {
    assert.deepEqual(
        githubDraftCreateArguments({
            tag: releaseTag,
            title: releaseTitle,
            draft: true,
            prerelease: false,
            generateNotes: true,
            notesStartTag: 'cem-ml-v3.4.4',
        }),
        [
            'release',
            'create',
            releaseTag,
            '--verify-tag',
            '--draft',
            '--title',
            releaseTitle,
            '--generate-notes',
            '--notes-start-tag',
            'cem-ml-v3.4.4',
        ],
    );
});

test('identical existing GitHub draft resumes without mutation', () => {
    const existing = githubDraft();
    const github = createFakeGithubReleaseClient(existing);
    const result = createOrResumePlatformReleaseDraft({
        authorized: true,
        version,
        sourceCommit,
        releaseTag,
        taggedCommit: sourceCommit,
        previousReleaseTag: 'cem-ml-v3.4.4',
        github,
    });

    assert.equal(result.action, 'resumed');
    assert.deepEqual(result.release, existing);
    assert.deepEqual(github.createRequests, []);
    assert.equal(github.publishCalls, 0);
});

for (const [label, release, message] of [
    ['wrong tag', githubDraft({ tagName: 'cem-ml-v3.4.4' }), /tag drift/],
    ['published release', githubDraft({ isDraft: false }), /must remain a draft/],
    ['wrong title', githubDraft({ name: 'Unexpected title' }), /title drift/],
    ['wrong prerelease state', githubDraft({ isPrerelease: true }), /prerelease drift/],
]) {
    test(`draft coordinator rejects ${label}`, () => {
        const github = createFakeGithubReleaseClient(release);
        assert.throws(
            () =>
                createOrResumePlatformReleaseDraft({
                    authorized: true,
                    version,
                    sourceCommit,
                    releaseTag,
                    taggedCommit: sourceCommit,
                    github,
                }),
            message,
        );
        assert.deepEqual(github.createRequests, []);
        assert.equal(github.publishCalls, 0);
    });
}

test('draft coordinator rejects a tag that does not resolve to the checked-out source commit', () => {
    const github = createFakeGithubReleaseClient();
    assert.throws(
        () =>
            createOrResumePlatformReleaseDraft({
                authorized: true,
                version,
                sourceCommit,
                releaseTag,
                taggedCommit: 'f'.repeat(40),
                github,
            }),
        /tagged source-commit drift/,
    );
    assert.deepEqual(github.createRequests, []);
});

test('draft coordinator rejects a manual tag outside the exact CEM-ML version contract', () => {
    const github = createFakeGithubReleaseClient();
    assert.throws(
        () =>
            createOrResumePlatformReleaseDraft({
                authorized: true,
                version,
                sourceCommit,
                releaseTag: `cem-ml-v${version}-unexpected`,
                taggedCommit: sourceCommit,
                github,
            }),
        /release tag drift/,
    );
    assert.deepEqual(github.createRequests, []);
});

test('draft coordinator remains inert without protected release authorization', () => {
    const github = createFakeGithubReleaseClient();
    assert.throws(
        () =>
            createOrResumePlatformReleaseDraft({
                authorized: false,
                version,
                sourceCommit,
                releaseTag,
                taggedCommit: sourceCommit,
                github,
            }),
        /draft creation is disabled/,
    );
    assert.deepEqual(github.createRequests, []);
});

test('CEM-ML workflow owns its tag family and generic publishing excludes it', () => {
    const workflow = readFileSync(resolve(workspaceRoot, '.github/workflows/cem-ml-release.yml'), 'utf8');
    const generic = readFileSync(resolve(workspaceRoot, '.github/workflows/publish.yml'), 'utf8');

    assert.match(workflow, /tags:\s*\n\s+- 'cem-ml-v\*'/);
    assert.match(workflow, /release_tag:\s*\n\s+description:/);
    assert.match(workflow, /group: cem-ml-release-/);
    assert.match(workflow, /name: cem-ml-release/);
    assert.match(workflow, /contents: write/);
    assert.match(workflow, /cem_ml:release:create-draft/);
    assert.match(workflow, /CEM_ML_PLATFORM_DRAFT: \$\{\{ vars\.CEM_ML_PLATFORM_DRAFT \}\}/);
    assert.doesNotMatch(workflow, /CEM_ML_PLATFORM_DRAFT: ['"]?1/);
    assert.doesNotMatch(workflow, /release edit|draft=false|nx release publish/);
    assert.match(generic, /- '!cem-ml-v\*'/);
});

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

function githubDraft(overrides = {}) {
    return {
        tagName: releaseTag,
        name: releaseTitle,
        isDraft: true,
        isPrerelease: false,
        assets: [],
        ...overrides,
    };
}

function createFakeGithubReleaseClient(initialRelease = null) {
    let release = initialRelease;
    const client = {
        createRequests: [],
        publishCalls: 0,
        view() {
            return release;
        },
        create(request) {
            client.createRequests.push(request);
            release = githubDraft({ isPrerelease: request.prerelease });
        },
    };
    return client;
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
