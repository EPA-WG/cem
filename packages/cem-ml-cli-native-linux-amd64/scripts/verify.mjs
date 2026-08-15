import assert from 'node:assert/strict';
import { existsSync, mkdtempSync, readFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { resolve, sep } from 'node:path';

import {
    artifactPath,
    assetNames,
    authoritativeVersion,
    buildRoot,
    capture,
    deployment,
    readJson,
    releaseAssetUrl,
    releaseTag,
    requireFile,
    run,
    sha256File,
} from './lib.mjs';

const version = authoritativeVersion();
const names = assetNames(version);
const requiredArtifacts = [
    names.archive,
    names.deb,
    names.checksum,
    names.sbom,
    names.capability,
    names.provenance,
    names.apt,
    names.releaseEntry,
    names.signing,
];
for (const filename of requiredArtifacts) requireFile(artifactPath(filename), filename);

const checksumLines = readFileSync(artifactPath(names.checksum), 'utf8').trim().split('\n');
const checksumEntries = new Map(
    checksumLines.map((line) => {
        const match = line.match(/^([0-9a-f]{64}) {2}([^/]+)$/);
        assert.ok(match, `invalid checksum line ${line}`);
        return [match[2], match[1]];
    }),
);
assert.equal(checksumEntries.size, checksumLines.length, 'checksum filenames must be unique');
for (const [filename, expected] of checksumEntries) {
    assert.equal(sha256File(requireFile(artifactPath(filename))), expected, filename);
}

const capability = readJson(artifactPath(names.capability));
assert.equal(capability.commonVersion, version);
assert.equal(capability.runtime, 'native');
assert.equal(capability.targetIdentity, deployment.rustTarget);
assert.match(capability.abiIdentity, /^cem-ml-native-cli-v1:/);

const buildMetadata = readJson(resolve(buildRoot, 'build-metadata.json'));
assert.equal(buildMetadata.commonVersion, version);
assert.equal(buildMetadata.sourceCommit.length, 40);
assert.equal(buildMetadata.rustTarget, deployment.rustTarget);
assert.equal(buildMetadata.runtimeIdentity, deployment.runtimeIdentity);
assert.equal(buildMetadata.capabilitySha256, sha256File(artifactPath(names.capability)));
assert.equal(buildMetadata.binarySha256, sha256File(resolve(buildRoot, deployment.rustBinary)));
assert.match(capture('file', [resolve(buildRoot, deployment.rustBinary)]), /ELF 64-bit.*x86-64/);

const sbom = readJson(artifactPath(names.sbom));
assert.equal(sbom.spdxVersion, 'SPDX-2.3');
assert.equal(sbom.dataLicense, 'CC0-1.0');
assert.equal(sbom.name, `cem-ml-${deployment.runtimeIdentity}-${version}`);
assert.match(sbom.documentNamespace, new RegExp(`/cem-ml/${escapeRegex(version)}/${deployment.runtimeIdentity}/`));
assert.ok(Array.isArray(sbom.packages));

const provenance = readJson(artifactPath(names.provenance));
assert.equal(provenance.status, 'unsigned-build-record');
assert.equal(provenance.buildDefinition.externalParameters.rustTarget, deployment.rustTarget);
assert.equal(provenance.buildDefinition.externalParameters.runtimeIdentity, deployment.runtimeIdentity);
assert.deepEqual(
    new Set(provenance.subject.map(({ filename }) => filename)),
    new Set([names.archive, names.deb, names.capability, names.sbom]),
);
for (const subject of provenance.subject) {
    assert.equal(subject.sha256, sha256File(artifactPath(subject.filename)), subject.filename);
}

const apt = readJson(artifactPath(names.apt));
assert.equal(apt.repository, deployment.debian.repository);
assert.equal(apt.version, version);
assert.equal(apt.architecture, deployment.debian.architecture);
assert.equal(apt.immutableSource.releaseTag, releaseTag(version));
assert.equal(apt.immutableSource.url, releaseAssetUrl(names.deb, version));
assert.equal(apt.immutableSource.sha256, sha256File(artifactPath(names.deb)));
assert.equal(apt.releaseMetadata.rebuildExecutable, false);

const releaseEntry = readJson(artifactPath(names.releaseEntry));
assert.equal(releaseEntry.commonVersion, version);
assert.equal(releaseEntry.runtimeIdentity, deployment.runtimeIdentity);
assert.equal(releaseEntry.targetIdentity, deployment.rustTarget);
assert.equal(releaseEntry.releaseTag, releaseTag(version));
assert.equal(releaseEntry.capabilityManifestDigest, sha256File(artifactPath(names.capability)));
for (const artifact of releaseEntry.artifacts) {
    assert.equal(artifact.sha256, sha256File(artifactPath(artifact.filename)), artifact.filename);
}

const signing = readJson(artifactPath(names.signing));
assert.equal(signing.commonVersion, version);
assert.equal(signing.runtimeIdentity, deployment.runtimeIdentity);
assert.equal(signing.checksumManifest.sha256, sha256File(artifactPath(names.checksum)));
if (signing.gpg.status === 'signed') {
    requireFile(artifactPath(names.checksumSignature));
    run('gpg', [
        '--verify',
        artifactPath(names.checksumSignature),
        artifactPath(names.checksum),
    ]);
}
if (process.env.CEM_ML_RELEASE_VERIFY === '1') {
    assert.equal(signing.publicationReady, true, 'release verification requires signatures and attestation');
}

assert.equal(capture('dpkg-deb', ['--field', artifactPath(names.deb), 'Package']).trim(), 'cem-ml');
assert.equal(capture('dpkg-deb', ['--field', artifactPath(names.deb), 'Version']).trim(), version);
assert.equal(
    capture('dpkg-deb', ['--field', artifactPath(names.deb), 'Architecture']).trim(),
    deployment.debian.architecture,
);
const debContents = capture('dpkg-deb', ['--contents', artifactPath(names.deb)]);
for (const path of [
    './usr/bin/cem-ml',
    './usr/share/cem-ml/capabilities.json',
    './usr/share/cem-ml/build-metadata.json',
    './usr/share/doc/cem-ml/copyright',
]) {
    assert.ok(debContents.includes(path), `Debian package is missing ${path}`);
}

const archiveContents = capture('tar', ['-tzf', artifactPath(names.archive)]);
for (const path of [
    `${names.base}/bin/cem-ml`,
    `${names.base}/share/cem-ml/capabilities.json`,
    `${names.base}/share/cem-ml/build-metadata.json`,
    `${names.base}/LICENSE`,
    `${names.base}/README.md`,
]) {
    assert.ok(archiveContents.split('\n').includes(path), `archive is missing ${path}`);
}

const extractionRoot = mkdtempSync(resolve(tmpdir(), 'cem-ml-native-linux-verify-'));
try {
    run('tar', ['-xzf', artifactPath(names.archive), '-C', extractionRoot]);
    const binary = resolve(extractionRoot, names.base, 'bin/cem-ml');
    assert.equal(capture(binary, ['version']).trim().split('\n')[0], `cem-ml ${version}`);
} finally {
    assert.ok(extractionRoot.startsWith(`${tmpdir()}${sep}cem-ml-native-linux-verify-`));
    rmSync(extractionRoot, { recursive: true, force: true });
}

assert.equal(existsSync(artifactPath(names.checksumSignature)), signing.gpg.status === 'signed');
console.log(
    `Verified ${deployment.runtimeIdentity} ${version}: archive, Debian package, checksums, SPDX SBOM, capability, provenance, signing, and APT records.`,
);

function escapeRegex(value) {
    return value.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
}
