import assert from 'node:assert/strict';
import { copyFileSync, existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { basename, resolve, sep } from 'node:path';

import {
    artifactPath,
    artifactRoot,
    assertNativeHost,
    assertPeAmd64,
    assertUnsignedAuthenticode,
    assertValidAuthenticode,
    assetNames,
    authoritativeVersion,
    capture,
    deployment,
    listFiles,
    productCode,
    readJson,
    releaseAssetUrl,
    releaseTag,
    requireFile,
    run,
    runResult,
    sha256File,
} from './lib.mjs';

assertNativeHost();
const version = authoritativeVersion();
const names = assetNames(version);
const requiredArtifacts = [
    names.archive,
    names.msi,
    names.checksum,
    names.sbom,
    names.capability,
    names.provenance,
    names.winget,
    names.wingetInstaller,
    names.wingetLocale,
    names.wingetVersion,
    names.releaseEntry,
    names.signing,
];
for (const filename of requiredArtifacts) requireFile(artifactPath(filename));

const signing = readJson(artifactPath(names.signing));
const allowedArtifacts = [...requiredArtifacts];
if (signing.publicationReady === true) allowedArtifacts.push(names.attestation);
assert.deepEqual(
    listFiles(artifactRoot)
        .map((path) => basename(path))
        .sort(),
    allowedArtifacts.sort(),
    'native Windows artifact set drifted',
);

const checksumLines = readFileSync(artifactPath(names.checksum), 'utf8').trim().split('\n');
const checksumEntries = new Map(
    checksumLines.map((line) => {
        const match = line.replace(/\r$/, '').match(/^([0-9a-f]{64}) {2}([^/\\]+)$/);
        assert.ok(match, `invalid checksum line ${line}`);
        return [match[2], match[1]];
    }),
);
assert.equal(checksumEntries.size, checksumLines.length, 'checksum filenames must be unique');
assert.deepEqual(
    [...checksumEntries.keys()],
    [...checksumEntries.keys()].sort(),
    'checksum manifest must be filename-sorted',
);
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
assert.equal(provenance.buildDefinition.externalParameters.wixVersion, deployment.wix.version);
assert.equal(provenance.buildDefinition.externalParameters.productCode, productCode(version));
assert.deepEqual(
    new Set(provenance.subject.map(({ filename }) => filename)),
    new Set([names.archive, names.msi, names.capability, names.sbom]),
);
for (const subject of provenance.subject) {
    assert.equal(subject.sha256, sha256File(artifactPath(subject.filename)), subject.filename);
}

const winget = readJson(artifactPath(names.winget));
assert.equal(winget.repository, deployment.windowsInstaller.wingetRepository);
assert.equal(winget.packageIdentifier, deployment.windowsInstaller.packageIdentifier);
assert.equal(winget.version, version);
assert.equal(winget.immutableSource.releaseTag, releaseTag(version));
assert.equal(winget.immutableSource.url, releaseAssetUrl(names.msi, version));
assert.equal(winget.immutableSource.sha256, sha256File(artifactPath(names.msi)));
assert.equal(winget.immutableSource.productCode, productCode(version));
for (const manifest of winget.manifestProjection.files) {
    assert.equal(manifest.sha256, sha256File(artifactPath(manifest.filename)), manifest.filename);
    assert.match(manifest.repositoryPath, new RegExp(`/${escapeRegex(version)}/`));
}
assert.equal(winget.manifestProjection.rebuildExecutable, false);

const installerManifest = readFileSync(artifactPath(names.wingetInstaller), 'utf8');
const wingetManifestVersion = escapeRegex(deployment.windowsInstaller.wingetManifestVersion);
assert.match(
    installerManifest,
    new RegExp(
        `^# yaml-language-server: \\$schema=https://aka\\.ms/winget-manifest\\.installer\\.${wingetManifestVersion}\\.schema\\.json$`,
        'm',
    ),
);
assert.match(installerManifest, new RegExp(`^ManifestVersion: ${wingetManifestVersion}$`, 'm'));
assert.match(installerManifest, /InstallerType: wix/);
assert.match(installerManifest, /Architecture: x64/);
assert.match(installerManifest, /ElevationRequirement: elevationRequired/);
assert.match(installerManifest, new RegExp(`InstallerSha256: ${sha256File(artifactPath(names.msi)).toUpperCase()}`));
assert.match(installerManifest, new RegExp(`ProductCode: '${escapeRegex(productCode(version))}'`));
assert.match(
    installerManifest,
    new RegExp(`^    UpgradeCode: '${escapeRegex(deployment.windowsInstaller.upgradeCode)}'$`, 'm'),
);
assert.doesNotMatch(installerManifest, /releases\/latest|archive\/refs\/heads/);

const releaseEntry = readJson(artifactPath(names.releaseEntry));
assert.equal(releaseEntry.commonVersion, version);
assert.equal(releaseEntry.runtimeIdentity, deployment.runtimeIdentity);
assert.equal(releaseEntry.targetIdentity, deployment.rustTarget);
assert.equal(releaseEntry.releaseTag, releaseTag(version));
assert.equal(releaseEntry.capabilityManifestDigest, sha256File(artifactPath(names.capability)));
assert.equal(releaseEntry.installerIdentity.productCode, productCode(version));
for (const artifact of releaseEntry.artifacts) {
    assert.equal(artifact.sha256, sha256File(artifactPath(artifact.filename)), artifact.filename);
}

assert.equal(signing.commonVersion, version);
assert.equal(signing.runtimeIdentity, deployment.runtimeIdentity);
assert.equal(signing.checksumManifest.sha256, sha256File(artifactPath(names.checksum)));
assert.equal(signing.artifactSigning.provider, 'Microsoft Artifact Signing');
assert.equal(signing.artifactSigning.trustModel, 'public-trust');
assert.equal(signing.artifactSigning.installer.sha256, sha256File(artifactPath(names.msi)));

const verifyRoot = mkdtempSync(resolve(tmpdir(), 'cem-ml-native-windows-verify-'));
try {
    const manifestRoot = resolve(verifyRoot, 'winget');
    const identifier = deployment.windowsInstaller.packageIdentifier;
    mkdirSync(manifestRoot, { recursive: true });
    copyFileSync(artifactPath(names.wingetInstaller), resolve(manifestRoot, `${identifier}.installer.yaml`));
    copyFileSync(artifactPath(names.wingetLocale), resolve(manifestRoot, `${identifier}.locale.en-US.yaml`));
    copyFileSync(artifactPath(names.wingetVersion), resolve(manifestRoot, `${identifier}.yaml`));
    run('winget.exe', ['validate', '--manifest', manifestRoot, '--disable-interactivity']);

    const wix = process.env.CEM_ML_WIX?.trim() || 'wix.exe';
    run(wix, ['msi', 'validate', artifactPath(names.msi)]);

    const zipRoot = resolve(verifyRoot, 'zip');
    expandArchive(artifactPath(names.archive), zipRoot);
    const zipBinary = requireFile(resolve(zipRoot, names.base, 'bin/cem-ml.exe'));
    const zipMetadata = readJson(requireFile(resolve(zipRoot, names.base, 'share/cem-ml/build-metadata.json')));
    verifyPackagedBinary(zipBinary, zipMetadata);

    const msiRoot = resolve(verifyRoot, 'msi');
    const msiLog = resolve(verifyRoot, 'administrative-install.log');
    runMsi(['/a', artifactPath(names.msi), '/qn', '/norestart', `TARGETDIR=${msiRoot}`, '/l*v', msiLog]);
    const msiFiles = listFiles(msiRoot);
    const msiBinaries = msiFiles.filter((path) => basename(path).toLowerCase() === 'cem-ml.exe');
    assert.equal(msiBinaries.length, 1, 'MSI administrative image must contain one cem-ml.exe');
    const msiMetadataFiles = msiFiles.filter((path) => basename(path) === 'build-metadata.json');
    assert.equal(msiMetadataFiles.length, 1, 'MSI administrative image must contain build metadata');
    const msiBinary = msiBinaries[0];
    const msiMetadata = readJson(msiMetadataFiles[0]);
    verifyPackagedBinary(msiBinary, msiMetadata);
    assert.equal(sha256File(msiBinary), sha256File(zipBinary), 'ZIP and MSI executable payloads differ');

    if (signing.artifactReady === true) {
        assertValidAuthenticode(zipBinary, 'ZIP executable');
        assertValidAuthenticode(msiBinary, 'MSI executable');
        assertValidAuthenticode(artifactPath(names.msi), 'MSI package');
        assert.ok(
            ['artifact-signing-staged', 'release'].includes(signing.mode),
            `unexpected signed verification mode ${signing.mode}`,
        );
    } else {
        assertUnsignedAuthenticode(zipBinary, 'ZIP executable');
        assertUnsignedAuthenticode(msiBinary, 'MSI executable');
        assertUnsignedAuthenticode(artifactPath(names.msi), 'MSI package');
        assert.equal(signing.mode, 'unsigned-local');
    }
} finally {
    assert.ok(verifyRoot.startsWith(`${tmpdir()}${sep}cem-ml-native-windows-verify-`));
    rmSync(verifyRoot, { recursive: true, force: true });
}

if (process.env.CEM_ML_RELEASE_VERIFY === '1') {
    assert.equal(signing.artifactReady, true, 'release verification requires Artifact Signing');
    assert.equal(signing.publicationReady, true, 'release verification requires attestation verification');
    assert.equal(existsSync(artifactPath(names.attestation)), true, 'release attestation bundle is missing');
}
assert.equal(existsSync(artifactPath(names.attestation)), signing.publicationReady === true);
console.log(
    `Verified ${deployment.runtimeIdentity} ${version}: ZIP, WiX MSI, checksums, SPDX SBOM, ` +
        'capability, provenance, Authenticode state, and WinGet manifests.',
);

function verifyPackagedBinary(binary, metadata) {
    assertPeAmd64(binary);
    assert.equal(metadata.commonVersion, version);
    assert.equal(metadata.sourceCommit.length, 40);
    assert.equal(metadata.rustTarget, deployment.rustTarget);
    assert.equal(metadata.runtimeIdentity, deployment.runtimeIdentity);
    assert.equal(metadata.capabilitySha256, sha256File(artifactPath(names.capability)));
    assert.equal(metadata.binarySha256, sha256File(binary));
    const reported = capture(binary, ['version']).trim().split('\n')[0].replace(/\r$/, '');
    assert.equal(reported, `cem-ml ${version}`);
}

function expandArchive(archive, destination) {
    const result = runResult(
        'pwsh.exe',
        [
            '-NoLogo',
            '-NoProfile',
            '-NonInteractive',
            '-Command',
            'Expand-Archive -LiteralPath $env:CEM_ML_ARCHIVE_PATH -DestinationPath $env:CEM_ML_ARCHIVE_DESTINATION -Force',
        ],
        {
            env: {
                CEM_ML_ARCHIVE_PATH: archive,
                CEM_ML_ARCHIVE_DESTINATION: destination,
            },
        },
    );
    if (result.status !== 0) {
        throw new Error(`Expand-Archive failed: ${result.stderr || result.stdout || result.error?.message}`);
    }
}

function runMsi(args) {
    const result = runResult('msiexec.exe', args, { stdio: 'pipe' });
    if (![0, 1641, 3010].includes(result.status)) {
        throw new Error(`msiexec ${args.join(' ')} failed: ${result.stderr || result.stdout || result.status}`);
    }
}

function escapeRegex(value) {
    return value.replace(/[.*+?^${}()|[\]\\]/g, '\\$&');
}
