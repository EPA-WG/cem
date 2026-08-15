import assert from 'node:assert/strict';
import { existsSync, mkdtempSync, readFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { resolve, sep } from 'node:path';

import { renderFormula } from './formula.mjs';
import {
    artifactPath,
    assetNames,
    assertNativeHost,
    authoritativeVersion,
    capture,
    captureCombined,
    deployment,
    readJson,
    releaseAssetUrl,
    releaseTag,
    requireFile,
    run,
    sha256File,
} from './lib.mjs';

assertNativeHost();
const version = authoritativeVersion();
const names = assetNames(version);
const requiredArtifacts = [
    names.archive,
    names.checksum,
    names.sbom,
    names.capability,
    names.provenance,
    names.homebrew,
    names.formula,
    names.releaseEntry,
    names.signing,
    names.notarization,
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

const sbom = readJson(artifactPath(names.sbom));
assert.equal(sbom.spdxVersion, 'SPDX-2.3');
assert.equal(sbom.dataLicense, 'CC0-1.0');
assert.equal(sbom.name, `cem-ml-${deployment.runtimeIdentity}-${version}`);
assert.match(sbom.documentNamespace, new RegExp(`/cem-ml/${escapeRegex(version)}/${deployment.runtimeIdentity}/`));
assert.ok(Array.isArray(sbom.packages));

const provenance = readJson(artifactPath(names.provenance));
assert.equal(provenance.buildDefinition.externalParameters.rustTarget, deployment.rustTarget);
assert.equal(provenance.buildDefinition.externalParameters.runtimeIdentity, deployment.runtimeIdentity);
assert.deepEqual(
    new Set(provenance.subject.map(({ filename }) => filename)),
    new Set([names.archive, names.capability, names.sbom]),
);
for (const subject of provenance.subject) {
    assert.equal(subject.sha256, sha256File(artifactPath(subject.filename)), subject.filename);
}

const archiveSha256 = sha256File(artifactPath(names.archive));
const formula = readFileSync(artifactPath(names.formula), 'utf8');
assert.equal(
    formula,
    renderFormula({
        archiveSha256,
        archiveUrl: releaseAssetUrl(names.archive, version),
        version,
    }),
);
assert.doesNotMatch(formula, /releases\/latest|archive\/refs\/heads/);
run('ruby', ['-c', artifactPath(names.formula)]);

const homebrew = readJson(artifactPath(names.homebrew));
assert.equal(homebrew.repository, deployment.homebrew.repository);
assert.equal(homebrew.repositoryPath, deployment.homebrew.repositoryPath);
assert.equal(homebrew.formula, deployment.homebrew.formula);
assert.equal(homebrew.version, version);
assert.equal(homebrew.architecture, deployment.host.architecture);
assert.equal(homebrew.immutableSource.releaseTag, releaseTag(version));
assert.equal(homebrew.immutableSource.url, releaseAssetUrl(names.archive, version));
assert.equal(homebrew.immutableSource.sha256, archiveSha256);
assert.equal(homebrew.formulaProjection.sha256, sha256File(artifactPath(names.formula)));
assert.equal(homebrew.formulaProjection.rebuildExecutable, false);

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
const notarization = readJson(artifactPath(names.notarization));
assert.equal(signing.commonVersion, version);
assert.equal(signing.runtimeIdentity, deployment.runtimeIdentity);
assert.equal(signing.checksumManifest.sha256, sha256File(artifactPath(names.checksum)));
assert.equal(notarization.commonVersion, version);
assert.equal(notarization.runtimeIdentity, deployment.runtimeIdentity);

const extractionRoot = mkdtempSync(resolve(tmpdir(), 'cem-ml-native-macos-verify-'));
try {
    run('tar', ['-xzf', artifactPath(names.archive), '-C', extractionRoot]);
    const binary = resolve(extractionRoot, names.base, 'bin/cem-ml');
    const metadata = readJson(resolve(extractionRoot, names.base, 'share/cem-ml/build-metadata.json'));
    assert.equal(metadata.commonVersion, version);
    assert.equal(metadata.sourceCommit.length, 40);
    assert.equal(metadata.rustTarget, deployment.rustTarget);
    assert.equal(metadata.runtimeIdentity, deployment.runtimeIdentity);
    assert.equal(metadata.capabilitySha256, sha256File(artifactPath(names.capability)));
    assert.equal(metadata.binarySha256, sha256File(binary));
    assert.match(capture('file', [binary]), /Mach-O 64-bit executable arm64/);
    assert.equal(capture('lipo', ['-archs', binary]).trim(), 'arm64');
    run('codesign', ['--verify', '--strict', '--verbose=2', binary]);
    const signatureDetails = captureCombined('codesign', ['-d', '--verbose=4', binary]);
    if (signing.apple.status === 'developer-id-signed') {
        assert.match(signatureDetails, /Authority=Developer ID Application:/);
        assert.equal(signing.appleReady, true);
        assert.equal(notarization.status, 'Accepted');
        assert.equal(notarization.submittedBinarySha256, sha256File(binary));
        const assessment = captureCombined('spctl', ['--assess', '--type', 'execute', '--verbose=4', binary]);
        assert.match(assessment, /source=Notarized Developer ID/);
    } else {
        assert.equal(signing.apple.status, 'adhoc-local');
        assert.equal(signing.appleReady, false);
        assert.equal(notarization.status, 'awaiting-release-credentials');
        assert.match(signatureDetails, /Signature=adhoc/);
    }
    assert.equal(capture(binary, ['version']).trim().split('\n')[0], `cem-ml ${version}`);
} finally {
    assert.ok(extractionRoot.startsWith(`${tmpdir()}${sep}cem-ml-native-macos-verify-`));
    rmSync(extractionRoot, { recursive: true, force: true });
}

if (process.env.CEM_ML_RELEASE_VERIFY === '1') {
    assert.equal(signing.appleReady, true, 'release verification requires Apple signing/notarization');
}
assert.equal(existsSync(artifactPath(names.attestation)), signing.publicationReady === true);
console.log(
    `Verified ${deployment.runtimeIdentity} ${version}: archive, checksums, SPDX SBOM, capability, ` +
        'provenance, Apple signing/notarization state, and Homebrew records.',
);

function escapeRegex(value) {
    return value.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
}
