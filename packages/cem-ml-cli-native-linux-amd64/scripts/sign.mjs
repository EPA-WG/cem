import { copyFileSync, rmSync } from 'node:fs';

import {
    artifactPath,
    assetNames,
    authoritativeVersion,
    deployment,
    releaseTag,
    requireFile,
    run,
    sha256File,
    writeJson,
} from './lib.mjs';

const version = authoritativeVersion();
const names = assetNames(version);
const checksum = requireFile(artifactPath(names.checksum), 'native checksum manifest');
const signature = artifactPath(names.checksumSignature);
const suppliedAttestation = process.env.CEM_ML_GITHUB_ATTESTATION_BUNDLE;
const releaseKey = process.env.CEM_ML_RELEASE_GPG_KEY;
rmSync(signature, { force: true });
rmSync(artifactPath(names.attestation), { force: true });

let gpg = { status: 'awaiting-release-credentials', signature: null, sha256: null };
if (releaseKey !== undefined && releaseKey.trim().length > 0) {
    run('gpg', [
        '--batch',
        '--yes',
        '--armor',
        '--local-user',
        releaseKey,
        '--output',
        signature,
        '--detach-sign',
        checksum,
    ]);
    run('gpg', ['--verify', signature, checksum]);
    gpg = {
        status: 'signed',
        signature: names.checksumSignature,
        sha256: sha256File(signature),
    };
}

let attestation = { status: 'awaiting-github-oidc', bundle: null, sha256: null };
if (suppliedAttestation !== undefined) {
    copyFileSync(requireFile(suppliedAttestation, 'GitHub attestation bundle'), artifactPath(names.attestation));
    attestation = {
        status: 'supplied',
        bundle: names.attestation,
        sha256: sha256File(artifactPath(names.attestation)),
    };
}

const publicationReady = gpg.status === 'signed' && attestation.status === 'supplied';
writeJson(artifactPath(names.signing), {
    schemaVersion: 1,
    product: 'cem-ml',
    commonVersion: version,
    runtimeIdentity: deployment.runtimeIdentity,
    releaseTag: releaseTag(version),
    checksumManifest: {
        filename: names.checksum,
        sha256: sha256File(checksum),
    },
    gpg,
    githubArtifactAttestation: attestation,
    aptReleaseMetadata: {
        repository: deployment.debian.repository,
        requiredSignature: 'EPA-WG release GPG identity',
    },
    publicationReady,
    mode: publicationReady ? 'release' : 'unsigned-local',
});

if (process.env.CEM_ML_RELEASE_SIGNING === 'required' && !publicationReady) {
    throw new Error(
        'release signing requires CEM_ML_RELEASE_GPG_KEY and CEM_ML_GITHUB_ATTESTATION_BUNDLE',
    );
}
console.log(
    publicationReady
        ? `Signed ${deployment.runtimeIdentity} ${version} release inputs.`
        : `Recorded deterministic unsigned-local signing state for ${deployment.runtimeIdentity} ${version}.`,
);
