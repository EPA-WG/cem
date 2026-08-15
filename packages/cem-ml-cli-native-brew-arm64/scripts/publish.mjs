import { copyFileSync, readdirSync } from 'node:fs';

import {
    artifactPath,
    artifactRoot,
    assetNames,
    assertNativeHost,
    authoritativeVersion,
    capture,
    deployment,
    readJson,
    releaseTag,
    requireFile,
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
if (signing.appleReady !== true) {
    throw new Error('native macOS publication requires Developer ID signing and accepted notarization');
}
const suppliedAttestation = process.env.CEM_ML_GITHUB_ATTESTATION_BUNDLE;
if (suppliedAttestation === undefined) {
    throw new Error('native macOS publication requires CEM_ML_GITHUB_ATTESTATION_BUNDLE');
}
requireFile(suppliedAttestation, 'GitHub artifact-attestation bundle');
run('gh', [
    'attestation',
    'verify',
    artifactPath(names.archive),
    '--repo',
    'EPA-WG/cem',
    '--bundle',
    suppliedAttestation,
]);
copyFileSync(suppliedAttestation, artifactPath(names.attestation));
signing.githubArtifactAttestation = {
    status: 'verified',
    bundle: names.attestation,
    sha256: sha256File(artifactPath(names.attestation)),
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
console.log(
    `Uploaded ${assets.length} immutable ${deployment.runtimeIdentity} assets to draft release ${tag}; ` +
        `${deployment.homebrew.repository} consumes ${names.formula}.`,
);
