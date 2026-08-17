import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { spawnSync } from 'node:child_process';
import {
    copyFileSync,
    mkdirSync,
    mkdtempSync,
    readFileSync,
    readdirSync,
    rmSync,
    statSync,
    writeFileSync,
} from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, resolve } from 'node:path';
import { fileURLToPath } from 'node:url';

const scriptPath = fileURLToPath(import.meta.url);
const defaultWorkspaceRoot = resolve(dirname(scriptPath), '../..');

export const expectedReleaseUnits = Object.freeze([
    {
        identity: '@epa-wg/cem-ml',
        root: 'dist/packages/cem-ml-npm/artifacts',
        targets: ['wasm32-unknown-unknown:nodejs', 'wasm32-unknown-unknown:web'],
    },
    {
        identity: '@epa-wg/cem-ml-cli',
        root: 'dist/packages/cem-ml-cli-npm/artifacts',
        targets: ['wasm32-unknown-unknown:nodejs', 'wasm32-unknown-unknown:web'],
    },
    {
        identity: 'native-linux-amd64',
        root: 'dist/packages/cem-ml-cli-native-linux-amd64/artifacts',
        target: 'x86_64-unknown-linux-gnu',
        channel: 'apt',
    },
    {
        identity: 'native-macos-arm64',
        root: 'dist/packages/cem-ml-cli-native-brew-arm64/artifacts',
        target: 'aarch64-apple-darwin',
        channel: 'homebrew',
    },
    {
        identity: 'native-windows-amd64',
        root: 'dist/packages/cem-ml-cli-native-windows-amd64/artifacts',
        target: 'x86_64-pc-windows-msvc',
        channel: 'winget',
    },
]);

export const ciReleaseUnits = Object.freeze(expectedReleaseUnits.slice(0, 3));

export function createOrResumePlatformReleaseDraft({
    workspaceRoot = defaultWorkspaceRoot,
    authorized = process.env.CEM_ML_PLATFORM_DRAFT === '1',
    version,
    sourceCommit,
    releaseTag,
    taggedCommit,
    previousReleaseTag,
    github,
} = {}) {
    if (!authorized) {
        throw new Error('draft creation is disabled; set CEM_ML_PLATFORM_DRAFT=1 in the protected release job');
    }
    version ??= authoritativeVersion(workspaceRoot);
    sourceCommit ??= gitSourceCommit(workspaceRoot);
    releaseTag ??= process.env.CEM_ML_RELEASE_TAG ?? `cem-ml-v${version}`;
    taggedCommit ??= gitTagSourceCommit(workspaceRoot, releaseTag);
    previousReleaseTag ??= previousPlatformReleaseTag(workspaceRoot, sourceCommit);
    github ??= createGithubReleaseClient(workspaceRoot);

    assert.equal(releaseTag, `cem-ml-v${version}`, 'CEM-ML release tag drift');
    assert.match(sourceCommit, /^[a-f0-9]{40,64}$/, 'invalid checked-out source commit');
    assert.equal(taggedCommit, sourceCommit, 'CEM-ML tagged source-commit drift');
    if (previousReleaseTag !== undefined) {
        assert.notEqual(previousReleaseTag, releaseTag, 'generated notes must start at a previous CEM-ML tag');
    }

    const title = `CEM-ML ${version}`;
    const prerelease = version.includes('-');
    let release = github.view(releaseTag);
    let action = 'resumed';
    if (release === null) {
        github.create({
            tag: releaseTag,
            title,
            draft: true,
            prerelease,
            generateNotes: true,
            notesStartTag: previousReleaseTag,
        });
        release = github.view(releaseTag);
        assert.ok(release, `created GitHub draft ${releaseTag} could not be read back`);
        action = 'created';
    }

    assert.equal(release.tagName, releaseTag, 'GitHub draft release tag drift');
    assert.equal(release.name, title, 'GitHub draft release title drift');
    assert.equal(release.isDraft, true, `${releaseTag} must remain a draft until protected promotion`);
    assert.equal(release.isPrerelease, prerelease, 'GitHub draft prerelease drift');
    assert.ok(Array.isArray(release.assets), 'GitHub draft asset listing is missing');
    return { action, release, releaseTag, sourceCommit, previousReleaseTag };
}

export function githubDraftCreateArguments({ tag, title, draft, prerelease, generateNotes, notesStartTag }) {
    assert.equal(draft, true, 'GitHub release coordinator may create drafts only');
    assert.equal(generateNotes, true, 'CEM-ML draft requires generated release notes');
    const args = ['release', 'create', tag, '--verify-tag', '--draft', '--title', title, '--generate-notes'];
    if (notesStartTag !== undefined) args.push('--notes-start-tag', notesStartTag);
    if (prerelease) args.push('--prerelease');
    return args;
}

export function stagePlatformRelease({
    workspaceRoot = defaultWorkspaceRoot,
    sourceCommit = gitSourceCommit(workspaceRoot),
    version = authoritativeVersion(workspaceRoot),
    publication = false,
    units = expectedReleaseUnits,
    outputRoot = resolve(workspaceRoot, 'dist/releases/cem-ml-platform', version),
} = {}) {
    if (publication) assertCleanSourceTree(workspaceRoot);
    const releaseTag = `cem-ml-v${version}`;
    const validated = units.map((unit) => {
        const root = resolve(workspaceRoot, unit.root);
        return validateReleaseUnit({ root, unit, version, sourceCommit, releaseTag, publication });
    });
    const assetsRoot = resolve(outputRoot, 'assets');
    assertGeneratedOutput(workspaceRoot, outputRoot);
    rmSync(outputRoot, { recursive: true, force: true });
    mkdirSync(assetsRoot, { recursive: true });

    const copied = new Map();
    for (const { root } of validated) {
        for (const filename of listFiles(root)) {
            if (!filename.startsWith(`cem-ml-${version}-`)) {
                throw new Error(`release asset is not version-qualified for ${version}: ${filename}`);
            }
            if (copied.has(filename)) throw new Error(`duplicate release asset filename: ${filename}`);
            copyFileSync(resolve(root, filename), resolve(assetsRoot, filename));
            copied.set(filename, artifactRecord(assetsRoot, filename));
        }
    }

    const indexName = `cem-ml-${version}.release-index.json`;
    const checksumName = `cem-ml-${version}.SHA256SUMS`;
    const index = {
        schemaVersion: 1,
        product: 'cem-ml',
        releaseGroup: 'cem-ml-platform',
        commonVersion: version,
        sourceCommit,
        releaseTag,
        publicationState: publication ? 'publication-ready' : 'verified-staged',
        units: validated
            .map(({ entry, entryFilename, identity }) => ({
                identity,
                releaseEntry: entryFilename,
                capabilityManifestDigest: entry.capabilityManifestDigest,
                checksumManifest: entry.checksumManifest,
                signingRecord: entry.signingRecord,
            }))
            .sort((left, right) => left.identity.localeCompare(right.identity)),
        assets: [...copied.values()].sort((left, right) => left.filename.localeCompare(right.filename)),
    };
    writeJson(resolve(assetsRoot, indexName), index);
    const checksummed = [...copied.keys(), indexName].sort();
    writeFileSync(
        resolve(assetsRoot, checksumName),
        `${checksummed.map((filename) => `${sha256File(resolve(assetsRoot, filename))}  ${filename}`).join('\n')}\n`,
    );
    verifyPlatformRelease({ workspaceRoot, outputRoot, version, sourceCommit, publication, units });
    return { outputRoot, assetsRoot, indexName, checksumName, index };
}

export function verifyPlatformRelease({
    workspaceRoot = defaultWorkspaceRoot,
    outputRoot,
    version = authoritativeVersion(workspaceRoot),
    sourceCommit = gitSourceCommit(workspaceRoot),
    publication = false,
    units = expectedReleaseUnits,
} = {}) {
    if (publication) assertCleanSourceTree(workspaceRoot);
    const releaseRoot = outputRoot ?? resolve(workspaceRoot, 'dist/releases/cem-ml-platform', version);
    const assetsRoot = resolve(releaseRoot, 'assets');
    const indexName = `cem-ml-${version}.release-index.json`;
    const checksumName = `cem-ml-${version}.SHA256SUMS`;
    const index = readJson(requireFile(resolve(assetsRoot, indexName), 'aggregate release index'));
    assert.equal(index.schemaVersion, 1, 'aggregate release index schema drift');
    assert.equal(index.releaseGroup, 'cem-ml-platform');
    assert.equal(index.commonVersion, version, 'aggregate release index version drift');
    assert.equal(index.sourceCommit, sourceCommit, 'aggregate release index source-commit drift');
    assert.equal(index.releaseTag, `cem-ml-v${version}`, 'aggregate release tag drift');
    assert.equal(index.publicationState, publication ? 'publication-ready' : 'verified-staged');
    assert.deepEqual(
        index.units.map(({ identity }) => identity).sort(),
        units.map(({ identity }) => identity).sort(),
        'aggregate release unit set drift',
    );
    const expectedAssets = new Map(index.assets.map((artifact) => [artifact.filename, artifact]));
    assert.equal(expectedAssets.size, index.assets.length, 'aggregate release index contains duplicate assets');
    for (const artifact of index.assets) verifyArtifactRecord(assetsRoot, artifact);

    for (const unit of units) {
        const summary = index.units.find(({ identity }) => identity === unit.identity);
        assert.ok(summary, `aggregate release index is missing ${unit.identity}`);
        validateReleaseUnit({
            root: assetsRoot,
            unit,
            version,
            sourceCommit,
            releaseTag: index.releaseTag,
            publication,
            entryFilename: summary.releaseEntry,
        });
    }
    const checksumEntries = readChecksumManifest(resolve(assetsRoot, checksumName));
    const expectedChecksummed = [...expectedAssets.keys(), indexName].sort();
    assert.deepEqual([...checksumEntries.keys()].sort(), expectedChecksummed, 'aggregate checksum asset set drift');
    for (const [filename, digest] of checksumEntries) {
        assert.equal(
            sha256File(requireFile(resolve(assetsRoot, filename))),
            digest,
            `${filename} aggregate checksum drift`,
        );
    }
    const actualFiles = listFiles(assetsRoot).sort();
    assert.deepEqual(actualFiles, [...expectedChecksummed, checksumName].sort(), 'unindexed aggregate release asset');
    return index;
}

export function verifyPlatformReleaseUnit({
    workspaceRoot = defaultWorkspaceRoot,
    identity,
    version,
    sourceCommit,
    releaseTag,
    taggedCommit,
} = {}) {
    const unit = expectedReleaseUnits.find((candidate) => candidate.identity === identity);
    if (!unit) throw new Error(`unsupported CEM-ML release unit: ${identity}`);
    version ??= authoritativeVersion(workspaceRoot);
    sourceCommit ??= gitSourceCommit(workspaceRoot);
    releaseTag ??= `cem-ml-v${version}`;
    taggedCommit ??= gitTagSourceCommit(workspaceRoot, releaseTag);
    assert.equal(releaseTag, `cem-ml-v${version}`, 'CEM-ML release-unit tag drift');
    assert.equal(taggedCommit, sourceCommit, 'CEM-ML release-unit source-commit drift');
    return validateReleaseUnit({
        root: resolve(workspaceRoot, unit.root),
        unit,
        version,
        sourceCommit,
        releaseTag,
        publication: true,
    });
}

export function uploadPlatformReleaseUnits({
    workspaceRoot = defaultWorkspaceRoot,
    units = ciReleaseUnits,
    authorized = process.env.CEM_ML_PLATFORM_UPLOAD === '1',
    version,
    sourceCommit,
    releaseTag,
    taggedCommit,
    github,
} = {}) {
    if (!authorized) {
        throw new Error('CI unit upload is disabled; set CEM_ML_PLATFORM_UPLOAD=1 in the protected release job');
    }
    version ??= authoritativeVersion(workspaceRoot);
    sourceCommit ??= gitSourceCommit(workspaceRoot);
    releaseTag ??= process.env.CEM_ML_RELEASE_TAG ?? `cem-ml-v${version}`;
    taggedCommit ??= gitTagSourceCommit(workspaceRoot, releaseTag);
    github ??= createGithubAssetClient(workspaceRoot);
    assert.equal(releaseTag, `cem-ml-v${version}`, 'CEM-ML CI-unit release tag drift');
    assert.equal(taggedCommit, sourceCommit, 'CEM-ML CI-unit tagged source-commit drift');

    const validated = units.map((unit) =>
        validateReleaseUnit({
            root: resolve(workspaceRoot, unit.root),
            unit,
            version,
            sourceCommit,
            releaseTag,
            publication: true,
        }),
    );
    const localAssets = new Map();
    const ownedBases = [];
    for (const { root, entryFilename } of validated) {
        ownedBases.push(entryFilename.replace(/\.release-index-entry\.json$/, ''));
        for (const filename of listFiles(root)) {
            if (!filename.startsWith(`cem-ml-${version}-`)) {
                throw new Error(`CI release asset is not version-qualified for ${version}: ${filename}`);
            }
            if (localAssets.has(filename)) throw new Error(`duplicate CI release asset filename: ${filename}`);
            localAssets.set(filename, resolve(root, filename));
        }
    }

    const release = github.view(releaseTag);
    assert.equal(release.tagName, releaseTag, 'GitHub draft release tag drift');
    assert.equal(release.isDraft, true, `${releaseTag} must remain a draft during CI unit upload`);
    const remoteNames = release.assets.map(({ name }) => name).sort();
    const unexpectedOwned = remoteNames.filter(
        (filename) => ownedBases.some((base) => filename.startsWith(`${base}.`)) && !localAssets.has(filename),
    );
    assert.deepEqual(unexpectedOwned, [], 'GitHub draft contains unexpected assets owned by CI release units');

    const existingRoot = mkdtempSync(resolve(tmpdir(), `cem-ml-${version}-ci-existing-`));
    try {
        for (const filename of remoteNames.filter((name) => localAssets.has(name))) {
            github.download(releaseTag, filename, existingRoot);
            assert.equal(
                sha256File(resolve(existingRoot, filename)),
                sha256File(localAssets.get(filename)),
                `existing CI draft asset is not immutable: ${filename}`,
            );
        }
    } finally {
        rmSync(existingRoot, { recursive: true, force: true });
    }

    const uploaded = [...localAssets.keys()].filter((filename) => !remoteNames.includes(filename));
    if (uploaded.length > 0) github.upload(releaseTag, uploaded.map((filename) => localAssets.get(filename)));

    const finalRelease = github.view(releaseTag);
    assert.equal(finalRelease.isDraft, true, `${releaseTag} was published during CI unit upload`);
    const finalNames = finalRelease.assets.map(({ name }) => name);
    const downloadRoot = mkdtempSync(resolve(tmpdir(), `cem-ml-${version}-ci-final-`));
    try {
        for (const [filename, localPath] of localAssets) {
            assert.ok(finalNames.includes(filename), `GitHub draft is missing CI asset ${filename}`);
            github.download(releaseTag, filename, downloadRoot);
            assert.equal(
                sha256File(resolve(downloadRoot, filename)),
                sha256File(localPath),
                `downloaded CI draft asset drift: ${filename}`,
            );
        }
    } finally {
        rmSync(downloadRoot, { recursive: true, force: true });
    }
    return {
        releaseTag,
        identities: validated.map(({ identity }) => identity),
        filenames: [...localAssets.keys()].sort(),
        uploaded: uploaded.sort(),
    };
}

export function uploadPlatformReleaseDraft({ workspaceRoot = defaultWorkspaceRoot } = {}) {
    if (process.env.CEM_ML_PLATFORM_UPLOAD !== '1') {
        throw new Error('draft upload is disabled; set CEM_ML_PLATFORM_UPLOAD=1 in the protected release job');
    }
    const version = authoritativeVersion(workspaceRoot);
    const sourceCommit = gitSourceCommit(workspaceRoot);
    const outputRoot = resolve(workspaceRoot, 'dist/releases/cem-ml-platform', version);
    const assetsRoot = resolve(outputRoot, 'assets');
    const index = verifyPlatformRelease({
        workspaceRoot,
        outputRoot,
        version,
        sourceCommit,
        publication: true,
    });
    const release = JSON.parse(
        capture('gh', ['release', 'view', index.releaseTag, '--json', 'assets,isDraft,tagName'], workspaceRoot),
    );
    assert.equal(release.tagName, index.releaseTag, 'GitHub draft release tag drift');
    assert.equal(release.isDraft, true, `${index.releaseTag} must remain a draft during complete asset staging`);
    const filenames = listFiles(assetsRoot);
    const remoteNames = release.assets.map(({ name }) => name).sort();
    const unexpectedRemote = remoteNames.filter((filename) => !filenames.includes(filename));
    assert.deepEqual(unexpectedRemote, [], 'draft GitHub Release contains assets outside the immutable stage');
    if (remoteNames.length > 0) {
        const existingRoot = mkdtempSync(resolve(tmpdir(), `cem-ml-${version}-existing-`));
        try {
            run('gh', ['release', 'download', index.releaseTag, '--dir', existingRoot], workspaceRoot);
            assert.deepEqual(listFiles(existingRoot), remoteNames, 'GitHub draft asset listing/download drift');
            for (const filename of remoteNames) {
                assert.equal(
                    sha256File(resolve(existingRoot, filename)),
                    sha256File(resolve(assetsRoot, filename)),
                    `existing draft asset is not immutable: ${filename}`,
                );
            }
        } finally {
            rmSync(existingRoot, { recursive: true, force: true });
        }
    }
    const missingNames = filenames.filter((filename) => !remoteNames.includes(filename));
    if (missingNames.length > 0) {
        run(
            'gh',
            ['release', 'upload', index.releaseTag, ...missingNames.map((filename) => resolve(assetsRoot, filename))],
            workspaceRoot,
        );
    }

    const downloadRoot = mkdtempSync(resolve(tmpdir(), `cem-ml-${version}-draft-`));
    try {
        run('gh', ['release', 'download', index.releaseTag, '--dir', downloadRoot], workspaceRoot);
        assert.deepEqual(listFiles(downloadRoot), filenames, 'draft GitHub Release asset set is incomplete or contains extras');
        for (const filename of filenames) {
            assert.equal(
                sha256File(resolve(downloadRoot, filename)),
                sha256File(resolve(assetsRoot, filename)),
                `downloaded draft asset drift: ${filename}`,
            );
        }
    } finally {
        rmSync(downloadRoot, { recursive: true, force: true });
    }
    return index;
}

export function validateReleaseUnit({
    root,
    unit,
    version,
    sourceCommit,
    releaseTag,
    publication = false,
    entryFilename,
}) {
    const releaseEntries = listFiles(root).filter((filename) => filename.endsWith('.release-index-entry.json'));
    const resolvedEntryFilename = entryFilename ?? releaseEntries[0];
    if (!resolvedEntryFilename || (!entryFilename && releaseEntries.length !== 1)) {
        throw new Error(`${unit.identity} must provide exactly one release-index entry`);
    }
    const entry = readJson(requireFile(resolve(root, resolvedEntryFilename), `${unit.identity} release entry`));
    const identity = entry.npmIdentity ?? entry.runtimeIdentity;
    assert.equal(identity, unit.identity, `${unit.identity} release identity drift`);
    assert.equal(entry.schemaVersion, 1, `${identity} release-index schema drift`);
    assert.equal(entry.product, 'cem-ml', `${identity} product drift`);
    assert.equal(entry.commonVersion, version, `${identity} version drift`);
    assert.equal(entry.sourceCommit, sourceCommit, `${identity} source-commit drift`);
    assert.equal(entry.releaseTag, releaseTag, `${identity} release-tag drift`);
    if (unit.target) assert.equal(entry.targetIdentity, unit.target, `${identity} target drift`);
    if (unit.targets) assert.deepEqual([...entry.targetIdentities].sort(), unit.targets, `${identity} targets drift`);

    assert.ok(Array.isArray(entry.artifacts) && entry.artifacts.length > 0, `${identity} has no release artifacts`);
    const artifactNames = new Set();
    for (const artifact of entry.artifacts) {
        assert.ok(!artifactNames.has(artifact.filename), `${identity} duplicate artifact ${artifact.filename}`);
        artifactNames.add(artifact.filename);
        assert.ok(artifact.filename.startsWith(`cem-ml-${version}-`), `${identity} unversioned artifact`);
        verifyArtifactRecord(root, artifact);
    }

    const capabilityArtifact = entry.artifacts.find(({ filename }) => filename.endsWith('.capabilities.json'));
    assert.ok(capabilityArtifact, `${identity} capability artifact is missing`);
    assert.equal(entry.capabilityManifestDigest, capabilityArtifact.sha256, `${identity} capability digest drift`);
    const capability = readJson(resolve(root, capabilityArtifact.filename));
    assertAllCommonVersions(capability, version, `${identity} capability`);

    const integrityArtifact = entry.artifacts.find(({ filename }) => filename.endsWith('.integrity.json'));
    if (entry.npmIdentity) {
        assert.ok(integrityArtifact, `${identity} integrity artifact is missing`);
        assert.equal(entry.integrityManifestDigest, integrityArtifact.sha256, `${identity} integrity digest drift`);
        assertAllCommonVersions(readJson(resolve(root, integrityArtifact.filename)), version, `${identity} integrity`);
    }

    const sbomArtifact = entry.artifacts.find(({ filename }) => filename.endsWith('.spdx.json'));
    assert.ok(sbomArtifact, `${identity} SPDX SBOM is missing`);
    const sbom = readJson(resolve(root, sbomArtifact.filename));
    assert.equal(sbom.spdxVersion, 'SPDX-2.3', `${identity} SBOM version drift`);
    assert.equal(sbom.dataLicense, 'CC0-1.0', `${identity} SBOM data license drift`);
    assert.ok(Array.isArray(sbom.packages), `${identity} SBOM package inventory is missing`);
    assert.ok(JSON.stringify(sbom).includes(version), `${identity} SBOM does not identify common version ${version}`);

    const provenanceArtifact = entry.artifacts.find(({ filename }) => filename.endsWith('.provenance.json'));
    assert.ok(provenanceArtifact, `${identity} provenance is missing`);
    const provenance = readJson(resolve(root, provenanceArtifact.filename));
    assert.equal(provenance.predicateType, 'https://slsa.dev/provenance/v1', `${identity} provenance type drift`);
    const commits = collectField(provenance, 'gitCommit');
    assert.deepEqual([...new Set(commits)], [sourceCommit], `${identity} provenance source-commit drift`);
    for (const subject of provenance.subject ?? []) verifyArtifactRecord(root, subject);

    const checksumPath = requireFile(resolve(root, entry.checksumManifest), `${identity} checksum manifest`);
    const checksumEntries = readChecksumManifest(checksumPath);
    assert.deepEqual(
        [...checksumEntries.keys()].sort(),
        [...artifactNames, resolvedEntryFilename].sort(),
        `${identity} checksum asset set drift`,
    );
    for (const [filename, digest] of checksumEntries) {
        assert.equal(
            sha256File(requireFile(resolve(root, filename))),
            digest,
            `${identity} checksum drift: ${filename}`,
        );
    }

    const signing = readJson(requireFile(resolve(root, entry.signingRecord), `${identity} signing record`));
    assert.equal(signing.commonVersion, version, `${identity} signing version drift`);
    assert.equal(signing.releaseTag, releaseTag, `${identity} signing tag drift`);
    assert.equal(signing.checksumManifest.filename, entry.checksumManifest, `${identity} signing checksum name drift`);
    assert.equal(signing.checksumManifest.sha256, sha256File(checksumPath), `${identity} signing checksum drift`);
    if (publication) assert.equal(signing.publicationReady, true, `${identity} is not publication-ready`);
    if (signing.publicationReady === true) validateAttestation(root, signing, identity);

    if (unit.channel) validateChannel(root, entry, unit.channel, version, releaseTag);
    return { root, entry, entryFilename: resolvedEntryFilename, identity };
}

function validateChannel(root, entry, expectedChannel, version, releaseTag) {
    const channelArtifact = entry.artifacts.find(({ filename }) => filename.endsWith(`.${expectedChannel}.json`));
    assert.ok(channelArtifact, `${entry.runtimeIdentity} ${expectedChannel} record is missing`);
    const channel = readJson(resolve(root, channelArtifact.filename));
    assert.equal(channel.channel, expectedChannel, `${entry.runtimeIdentity} channel drift`);
    assert.equal(channel.version, version, `${entry.runtimeIdentity} channel version drift`);
    assert.equal(channel.immutableSource.releaseTag, releaseTag, `${entry.runtimeIdentity} mutable channel tag`);
    assert.equal(
        channel.immutableSource.url,
        `https://github.com/EPA-WG/cem/releases/download/${releaseTag}/${channel.immutableSource.filename}`,
        `${entry.runtimeIdentity} channel URL is not immutable`,
    );
    const sourceArtifact = entry.artifacts.find(({ filename }) => filename === channel.immutableSource.filename);
    assert.ok(sourceArtifact, `${entry.runtimeIdentity} channel source is not indexed`);
    assert.equal(
        channel.immutableSource.sha256,
        sourceArtifact.sha256,
        `${entry.runtimeIdentity} channel digest drift`,
    );
}

function validateAttestation(root, signing, identity) {
    const attestation = signing.githubArtifactAttestation;
    assert.ok(attestation, `${identity} publication-ready signing record has no attestation`);
    assert.ok(['supplied', 'verified'].includes(attestation.status), `${identity} attestation is not verified`);
    assert.equal(
        attestation.sha256,
        sha256File(requireFile(resolve(root, attestation.bundle), `${identity} attestation bundle`)),
        `${identity} attestation digest drift`,
    );
}

function assertAllCommonVersions(value, version, label) {
    const versions = collectField(value, 'commonVersion');
    assert.ok(versions.length > 0, `${label} has no commonVersion`);
    assert.deepEqual([...new Set(versions)], [version], `${label} version drift`);
}

function collectField(value, field, results = []) {
    if (Array.isArray(value)) {
        for (const child of value) collectField(child, field, results);
    } else if (value && typeof value === 'object') {
        for (const [key, child] of Object.entries(value)) {
            if (key === field) results.push(child);
            collectField(child, field, results);
        }
    }
    return results;
}

function verifyArtifactRecord(root, artifact) {
    const path = requireFile(resolve(root, artifact.filename), artifact.filename);
    assert.equal(statSync(path).size, artifact.byteLength, `${artifact.filename} byte-length drift`);
    assert.equal(sha256File(path), artifact.sha256, `${artifact.filename} SHA-256 drift`);
}

function readChecksumManifest(path) {
    const entries = new Map();
    for (const line of readFileSync(path, 'utf8').trim().split('\n')) {
        const match = line.match(/^([a-f0-9]{64}) {2}([^/\\]+)$/);
        assert.ok(match, `invalid checksum line in ${path}: ${line}`);
        assert.ok(!entries.has(match[2]), `duplicate checksum filename in ${path}: ${match[2]}`);
        entries.set(match[2], match[1]);
    }
    return entries;
}

function authoritativeVersion(workspaceRoot) {
    const source = readFileSync(resolve(workspaceRoot, 'packages/cem_ml/Cargo.toml'), 'utf8');
    const version = source.match(/^\[package\][\s\S]*?^version\s*=\s*"([^"]+)"/m)?.[1];
    if (!version) throw new Error('cannot read the common CEM-ML Cargo version');
    return version;
}

function gitSourceCommit(workspaceRoot) {
    const result = spawnSync('git', ['rev-parse', 'HEAD'], { cwd: workspaceRoot, encoding: 'utf8', stdio: 'pipe' });
    if (result.status !== 0) throw new Error(`git rev-parse HEAD failed: ${result.stderr}`);
    return result.stdout.trim();
}

function gitTagSourceCommit(workspaceRoot, releaseTag) {
    const result = spawnSync('git', ['rev-parse', `${releaseTag}^{commit}`], {
        cwd: workspaceRoot,
        encoding: 'utf8',
        stdio: 'pipe',
    });
    if (result.status !== 0) throw new Error(`cannot resolve required release tag ${releaseTag}: ${result.stderr}`);
    return result.stdout.trim();
}

function previousPlatformReleaseTag(workspaceRoot, sourceCommit) {
    const result = spawnSync('git', ['describe', '--tags', '--abbrev=0', '--match', 'cem-ml-v*', `${sourceCommit}^`], {
        cwd: workspaceRoot,
        encoding: 'utf8',
        stdio: 'pipe',
    });
    if (result.status === 0) return result.stdout.trim();
    if (/cannot describe|No names found|unknown revision|bad revision/i.test(`${result.stdout}\n${result.stderr}`)) {
        return undefined;
    }
    throw new Error(`cannot resolve previous CEM-ML release tag: ${result.stderr}`);
}

function createGithubReleaseClient(workspaceRoot) {
    return {
        view(releaseTag) {
            const result = spawnSync(
                'gh',
                ['release', 'view', releaseTag, '--json', 'assets,isDraft,isPrerelease,name,tagName'],
                { cwd: workspaceRoot, encoding: 'utf8', stdio: 'pipe' },
            );
            if (result.status === 0) return JSON.parse(result.stdout);
            if (/release not found|HTTP 404|not found/i.test(`${result.stdout}\n${result.stderr}`)) return null;
            throw new Error(`gh release view ${releaseTag} failed: ${result.stderr}`);
        },
        create(request) {
            run('gh', githubDraftCreateArguments(request), workspaceRoot);
        },
    };
}

function createGithubAssetClient(workspaceRoot) {
    return {
        view(releaseTag) {
            return JSON.parse(
                capture('gh', ['release', 'view', releaseTag, '--json', 'assets,isDraft,tagName'], workspaceRoot),
            );
        },
        download(releaseTag, filename, destinationRoot) {
            run('gh', ['release', 'download', releaseTag, '--pattern', filename, '--dir', destinationRoot], workspaceRoot);
        },
        upload(releaseTag, paths) {
            run('gh', ['release', 'upload', releaseTag, ...paths], workspaceRoot);
        },
    };
}

function run(command, args, cwd) {
    const result = spawnSync(command, args, { cwd, encoding: 'utf8', stdio: 'inherit' });
    if (result.status !== 0) throw new Error(`${command} ${args.join(' ')} failed with status ${result.status}`);
}

function capture(command, args, cwd) {
    const result = spawnSync(command, args, { cwd, encoding: 'utf8', stdio: 'pipe' });
    if (result.status !== 0) throw new Error(`${command} ${args.join(' ')} failed: ${result.stderr}`);
    return result.stdout;
}

function assertCleanSourceTree(workspaceRoot) {
    const result = spawnSync('git', ['status', '--porcelain', '--untracked-files=all'], {
        cwd: workspaceRoot,
        encoding: 'utf8',
        stdio: 'pipe',
    });
    if (result.status !== 0) throw new Error(`git status failed: ${result.stderr}`);
    if (result.stdout.trim()) throw new Error('publication staging requires a clean source tree at the tagged commit');
}

function assertGeneratedOutput(workspaceRoot, outputRoot) {
    const allowedRoot = resolve(workspaceRoot, 'dist/releases/cem-ml-platform');
    const normalized = resolve(outputRoot);
    if (normalized !== allowedRoot && !normalized.startsWith(`${allowedRoot}/`)) {
        throw new Error(`refusing to reset non-release output path: ${normalized}`);
    }
}

function artifactRecord(root, filename) {
    const path = resolve(root, filename);
    return { filename, byteLength: statSync(path).size, sha256: sha256File(path) };
}

function listFiles(root) {
    return readdirSync(root, { withFileTypes: true })
        .filter((entry) => entry.isFile())
        .map(({ name }) => name)
        .sort();
}

function requireFile(path, label = path) {
    if (!statSync(path, { throwIfNoEntry: false })?.isFile()) throw new Error(`missing ${label}: ${path}`);
    return path;
}

function sha256File(path) {
    return createHash('sha256').update(readFileSync(path)).digest('hex');
}

function readJson(path) {
    return JSON.parse(readFileSync(path, 'utf8'));
}

function writeJson(path, value) {
    writeFileSync(path, `${JSON.stringify(value, null, 2)}\n`);
}

if (process.argv[1] && resolve(process.argv[1]) === scriptPath) {
    const command = process.argv[2];
    const args = process.argv.slice(3);
    const publication = args.includes('--publication');
    const standardCommand = ['create-draft', 'stage', 'verify', 'upload-draft', 'upload-ci-units'].includes(command);
    const verifyUnitCommand = command === 'verify-unit' && args.length === 1;
    if ((!standardCommand && !verifyUnitCommand) || (standardCommand && args.some((arg) => arg !== '--publication'))) {
        throw new Error(
            'usage: node tools/scripts/cem-ml-platform-release.mjs <create-draft|stage|verify|upload-draft|upload-ci-units> [--publication] | verify-unit <identity>',
        );
    }
    if (command === 'create-draft') {
        if (publication) throw new Error('create-draft never publishes; omit --publication');
        const result = createOrResumePlatformReleaseDraft();
        console.log(
            `${result.action === 'created' ? 'Created' : 'Resumed'} protected GitHub draft ${result.releaseTag}.`,
        );
    } else if (command === 'stage') {
        const result = stagePlatformRelease({ publication });
        console.log(
            `Staged ${result.index.assets.length} CEM-ML ${result.index.commonVersion} assets from five deployments.`,
        );
    } else if (command === 'verify') {
        const index = verifyPlatformRelease({ publication });
        console.log(`Verified immutable CEM-ML ${index.commonVersion} release stage across five deployments.`);
    } else if (command === 'upload-draft') {
        if (publication) throw new Error('upload-draft is always publication mode; omit --publication');
        const index = uploadPlatformReleaseDraft();
        console.log(`Uploaded and reverified the complete draft ${index.releaseTag} asset set.`);
    } else if (command === 'upload-ci-units') {
        if (publication) throw new Error('upload-ci-units validates publication-ready units implicitly');
        const result = uploadPlatformReleaseUnits();
        console.log(`Uploaded ${result.uploaded.length} missing assets for ${result.identities.join(', ')}.`);
    } else {
        const result = verifyPlatformReleaseUnit({ identity: args[0] });
        console.log(`Verified publication-ready CEM-ML release unit ${result.identity}.`);
    }
}
