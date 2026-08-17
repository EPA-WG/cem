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
    const publication = process.argv.includes('--publication');
    if (
        !['stage', 'verify', 'upload-draft'].includes(command) ||
        process.argv.slice(3).some((arg) => arg !== '--publication')
    ) {
        throw new Error(
            'usage: node tools/scripts/cem-ml-platform-release.mjs <stage|verify|upload-draft> [--publication]',
        );
    }
    if (command === 'stage') {
        const result = stagePlatformRelease({ publication });
        console.log(
            `Staged ${result.index.assets.length} CEM-ML ${result.index.commonVersion} assets from five deployments.`,
        );
    } else if (command === 'verify') {
        const index = verifyPlatformRelease({ publication });
        console.log(`Verified immutable CEM-ML ${index.commonVersion} release stage across five deployments.`);
    } else {
        if (publication) throw new Error('upload-draft is always publication mode; omit --publication');
        const index = uploadPlatformReleaseDraft();
        console.log(`Uploaded and reverified the complete draft ${index.releaseTag} asset set.`);
    }
}
