import { copyFileSync, readFileSync } from 'node:fs';

import {
    artifactPath,
    assetNames,
    assertNativeHost,
    authoritativeVersion,
    deployment,
    projectRoot,
    readJson,
    requireFile,
    run,
    sha256File,
    writeJson,
} from './lib.mjs';

assertNativeHost();
if (process.env.CEM_ML_RELEASE_SIGNING !== 'required') {
    throw new Error('native Windows release finalization requires CEM_ML_RELEASE_SIGNING=required');
}
const version = authoritativeVersion();
const names = assetNames(version);
const signing = readJson(requireFile(artifactPath(names.signing)));
if (signing.artifactReady !== true) {
    throw new Error('native Windows finalization requires Artifact Signing for the executable and MSI');
}
const suppliedAttestation = process.env.CEM_ML_GITHUB_ATTESTATION_BUNDLE;
if (!suppliedAttestation?.trim()) {
    throw new Error('native Windows finalization requires CEM_ML_GITHUB_ATTESTATION_BUNDLE');
}
requireFile(suppliedAttestation, 'GitHub artifact-attestation bundle');
const checksum = requireFile(artifactPath(names.checksum), 'native Windows checksum manifest');
for (const line of readFileSync(checksum, 'utf8').trim().split('\n')) {
    const match = line.match(/^[a-f0-9]{64} {2}([^/\\]+)$/);
    if (!match) throw new Error(`invalid native Windows checksum line: ${line}`);
    run('gh', [
        'attestation',
        'verify',
        artifactPath(match[1]),
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
run('node', ['scripts/verify.mjs'], {
    cwd: projectRoot,
    env: { CEM_ML_RELEASE_VERIFY: '1' },
});
console.log(`Finalized attested ${deployment.runtimeIdentity} ${version} release assets without publishing.`);
