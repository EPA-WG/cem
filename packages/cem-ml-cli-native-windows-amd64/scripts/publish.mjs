import { copyFileSync, readdirSync, rmSync } from 'node:fs';
import { resolve } from 'node:path';

import {
    artifactPath,
    artifactRoot,
    assertNativeHost,
    assertValidAuthenticode,
    assetNames,
    authoritativeVersion,
    capture,
    deployment,
    outputRoot,
    readJson,
    releaseTag,
    requireFile,
    resetDirectory,
    run,
    sha256File,
    writeJson,
} from './lib.mjs';

assertNativeHost();
if (process.env.CEM_ML_NATIVE_PUBLISH !== '1') {
    throw new Error('native publication is disabled; set CEM_ML_NATIVE_PUBLISH=1 in the protected release job');
}
const version = authoritativeVersion();
const names = assetNames(version);
const signing = readJson(requireFile(artifactPath(names.signing)));
if (signing.artifactReady !== true) {
    throw new Error('native Windows publication requires Artifact Signing for the executable and MSI');
}
const suppliedAttestation = process.env.CEM_ML_GITHUB_ATTESTATION_BUNDLE;
if (suppliedAttestation === undefined) {
    throw new Error('native Windows publication requires CEM_ML_GITHUB_ATTESTATION_BUNDLE');
}
requireFile(suppliedAttestation, 'GitHub artifact-attestation bundle');
for (const filename of [names.archive, names.msi]) {
    run('gh', [
        'attestation',
        'verify',
        artifactPath(filename),
        '--repo',
        'EPA-WG/cem',
        '--bundle',
        suppliedAttestation,
    ]);
}
copyFileSync(suppliedAttestation, artifactPath(names.attestation));
signing.githubArtifactAttestation = {
    status: 'verified',
    bundle: names.attestation,
    sha256: sha256File(artifactPath(names.attestation)),
    subjects: [names.archive, names.msi],
};
signing.publicationReady = true;
signing.mode = 'release';
writeJson(artifactPath(names.signing), signing);

const tag = releaseTag(version);
const release = JSON.parse(capture('gh', ['release', 'view', tag, '--json', 'isDraft,tagName']));
if (release.tagName !== tag || release.isDraft !== true) {
    throw new Error(`${tag} must already exist as a draft GitHub Release`);
}

const assets = readdirSync(artifactRoot)
    .filter((filename) => filename.startsWith(`${names.base}.`))
    .sort()
    .map((filename) => artifactPath(filename));
for (const asset of assets) requireFile(asset);
run('gh', ['release', 'upload', tag, ...assets]);

const downloadRoot = resolve(outputRoot, 'post-download');
resetDirectory(downloadRoot);
run('gh', ['release', 'download', tag, '--pattern', names.archive, '--pattern', names.msi, '--dir', downloadRoot]);
const downloadedArchive = requireFile(resolve(downloadRoot, names.archive));
const downloadedMsi = requireFile(resolve(downloadRoot, names.msi));
if (sha256File(downloadedArchive) !== sha256File(artifactPath(names.archive))) {
    throw new Error('downloaded ZIP digest does not match the staged release asset');
}
if (sha256File(downloadedMsi) !== sha256File(artifactPath(names.msi))) {
    throw new Error('downloaded MSI digest does not match the staged release asset');
}
const extractedRoot = resolve(downloadRoot, 'archive');
run(
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
            CEM_ML_ARCHIVE_PATH: downloadedArchive,
            CEM_ML_ARCHIVE_DESTINATION: extractedRoot,
        },
    },
);
const downloadedBinary = requireFile(resolve(extractedRoot, names.base, 'bin/cem-ml.exe'));
const binarySignature = assertValidAuthenticode(downloadedBinary, 'downloaded ZIP executable');
const msiSignature = assertValidAuthenticode(downloadedMsi, 'downloaded MSI');
signing.postDownloadVerification = {
    status: 'verified',
    archiveSha256: sha256File(downloadedArchive),
    installerSha256: sha256File(downloadedMsi),
    executableAuthenticodeStatus: binarySignature.status,
    executableTimeStamperThumbprint: binarySignature.timeStamperThumbprint,
    installerAuthenticodeStatus: msiSignature.status,
    installerTimeStamperThumbprint: msiSignature.timeStamperThumbprint,
};
writeJson(artifactPath(names.signing), signing);
run('gh', ['release', 'upload', tag, artifactPath(names.signing), '--clobber']);
rmSync(downloadRoot, { recursive: true, force: true });
console.log(
    `Uploaded ${assets.length} immutable ${deployment.runtimeIdentity} assets to draft release ${tag}; ` +
        `post-download Authenticode verification passed and ${deployment.windowsInstaller.wingetRepository} ` +
        `consumes the versioned manifest projections.`,
);
