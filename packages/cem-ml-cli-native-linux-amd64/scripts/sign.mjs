import { copyFileSync, readFileSync, rmSync } from 'node:fs';

import {
    artifactPath,
    assetNames,
    authoritativeVersion,
    deployment,
    releaseTag,
    releaseGpgSigningInvocation,
    releasePublicationReady,
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
const releasePassphrase = process.env.CEM_ML_RELEASE_GPG_PASSPHRASE;
const signingRequired = process.env.CEM_ML_RELEASE_SIGNING === 'required';
if (signingRequired && (!releaseKey?.trim() || !releasePassphrase || !suppliedAttestation)) {
    throw new Error(
        'release signing requires CEM_ML_RELEASE_GPG_KEY, CEM_ML_RELEASE_GPG_PASSPHRASE, and CEM_ML_GITHUB_ATTESTATION_BUNDLE',
    );
}
rmSync(signature, { force: true });
rmSync(artifactPath(names.attestation), { force: true });

let gpg = { status: 'awaiting-release-credentials', signature: null, sha256: null };
if (releaseKey !== undefined && releaseKey.trim().length > 0) {
    const invocation = releaseGpgSigningInvocation({
        releaseKey,
        passphrase: releasePassphrase,
        signature,
        checksum,
    });
    run('gpg', invocation.args, { input: invocation.input, stdio: invocation.stdio });
    run('gpg', ['--verify', signature, checksum]);
    gpg = {
        status: 'signed',
        signature: names.checksumSignature,
        sha256: sha256File(signature),
    };
}

let attestation = { status: 'awaiting-github-oidc', bundle: null, sha256: null };
if (suppliedAttestation !== undefined) {
    const bundle = requireFile(suppliedAttestation, 'GitHub attestation bundle');
    for (const line of readFileSync(checksum, 'utf8').trim().split('\n')) {
        const match = line.match(/^[a-f0-9]{64} {2}([^/\\]+)$/);
        if (!match) throw new Error(`invalid native checksum line: ${line}`);
        run('gh', ['attestation', 'verify', artifactPath(match[1]), '--repo', 'EPA-WG/cem', '--bundle', bundle]);
    }
    copyFileSync(bundle, artifactPath(names.attestation));
    attestation = {
        status: 'verified',
        bundle: names.attestation,
        sha256: sha256File(artifactPath(names.attestation)),
    };
}

const publicationReady = releasePublicationReady({
    gpgStatus: gpg.status,
    attestationStatus: attestation.status,
});
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

if (signingRequired && !publicationReady) {
    throw new Error(
        'release signing requires verified GPG and GitHub artifact-attestation evidence',
    );
}
console.log(
    publicationReady
        ? `Signed ${deployment.runtimeIdentity} ${version} release inputs.`
        : `Recorded deterministic unsigned-local signing state for ${deployment.runtimeIdentity} ${version}.`,
);
