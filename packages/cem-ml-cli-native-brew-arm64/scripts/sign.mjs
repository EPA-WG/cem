import { readFileSync, rmSync, writeFileSync } from 'node:fs';
import { resolve } from 'node:path';

import { assembleArtifacts } from './assemble.mjs';
import {
    artifactPath,
    assetNames,
    assertNativeHost,
    authoritativeVersion,
    buildRoot,
    capture,
    captureCombined,
    copyExecutable,
    deployment,
    outputRoot,
    readJson,
    releaseTag,
    resetDirectory,
    run,
    sha256File,
    writeJson,
} from './lib.mjs';

assertNativeHost();
const version = authoritativeVersion();
const names = assetNames(version);
const signingIdentity = process.env.CEM_ML_APPLE_SIGNING_IDENTITY?.trim();
const notaryProfile = process.env.CEM_ML_APPLE_NOTARY_PROFILE?.trim();
if ((signingIdentity === undefined) !== (notaryProfile === undefined)) {
    throw new Error(
        'Apple release signing requires both CEM_ML_APPLE_SIGNING_IDENTITY and CEM_ML_APPLE_NOTARY_PROFILE',
    );
}

if (signingIdentity === undefined) {
    writeJson(artifactPath(names.notarization), {
        schemaVersion: 1,
        commonVersion: version,
        runtimeIdentity: deployment.runtimeIdentity,
        status: 'awaiting-release-credentials',
        submissionFormat: 'zip',
        publishedArchive: names.archive,
        submittedBinarySha256: null,
        submissionId: null,
    });
    finalizeNotarization('staged-local');
    writeSigningRecord({
        apple: {
            status: 'adhoc-local',
            identity: null,
            binarySha256: sha256File(resolve(buildRoot, deployment.rustBinary)),
            hardenedRuntime: true,
            secureTimestamp: false,
        },
        appleReady: false,
    });
    if (process.env.CEM_ML_RELEASE_SIGNING === 'required') {
        throw new Error('release signing requires CEM_ML_APPLE_SIGNING_IDENTITY and CEM_ML_APPLE_NOTARY_PROFILE');
    }
    console.log(
        `Recorded deterministic ad-hoc signing/notarization state for ${deployment.runtimeIdentity} ${version}.`,
    );
} else {
    const signingRoot = resolve(outputRoot, 'signing-work');
    resetDirectory(signingRoot);
    const signedBinary = resolve(signingRoot, deployment.rustBinary);
    copyExecutable(resolve(buildRoot, deployment.rustBinary), signedBinary);
    run('codesign', ['--force', '--sign', signingIdentity, '--timestamp', '--options', 'runtime', signedBinary]);
    run('codesign', ['--verify', '--strict', '--verbose=2', signedBinary]);
    const signatureDetails = captureCombined('codesign', ['-d', '--verbose=4', signedBinary]).trim();

    await assembleArtifacts({
        binaryPath: signedBinary,
        codeSignature: 'developer-id-hardened-runtime',
        distributionMode: 'developer-id-signed',
    });

    const submission = resolve(outputRoot, `${names.base}.notary-submission.zip`);
    rmSync(submission, { force: true });
    run('ditto', ['-c', '-k', '--keepParent', signedBinary, submission]);
    const response = JSON.parse(
        capture('xcrun', [
            'notarytool',
            'submit',
            submission,
            '--keychain-profile',
            notaryProfile,
            '--wait',
            '--output-format',
            'json',
        ]),
    );
    if (response.status !== 'Accepted') {
        throw new Error(`Apple notarization returned ${response.status ?? 'an unknown status'}`);
    }
    const notaryLog = JSON.parse(
        capture('xcrun', ['notarytool', 'log', response.id, '--keychain-profile', notaryProfile]),
    );
    const assessment = captureCombined('spctl', ['--assess', '--type', 'execute', '--verbose=4', signedBinary]).trim();
    if (!/source=Notarized Developer ID/.test(assessment)) {
        throw new Error(`Gatekeeper did not report a notarized Developer ID: ${assessment}`);
    }
    writeJson(artifactPath(names.notarization), {
        schemaVersion: 1,
        commonVersion: version,
        runtimeIdentity: deployment.runtimeIdentity,
        status: response.status,
        submissionFormat: 'zip',
        publishedArchive: names.archive,
        publishedArchiveSha256: sha256File(artifactPath(names.archive)),
        submittedBinarySha256: sha256File(signedBinary),
        submissionContainerSha256: sha256File(submission),
        submissionId: response.id,
        gatekeeperAssessment: assessment,
        log: notaryLog,
    });
    finalizeNotarization('signed-staged');
    writeSigningRecord({
        apple: {
            status: 'developer-id-signed',
            identity: signingIdentity,
            binarySha256: sha256File(signedBinary),
            hardenedRuntime: true,
            secureTimestamp: true,
            details: signatureDetails,
        },
        appleReady: true,
    });
    rmSync(signingRoot, { recursive: true, force: true });
    rmSync(submission, { force: true });
    console.log(`Developer ID signed and notarized ${deployment.runtimeIdentity} ${version} release inputs.`);
}

function finalizeNotarization(publicationState) {
    const releaseEntry = readJson(artifactPath(names.releaseEntry));
    releaseEntry.artifacts = [
        ...releaseEntry.artifacts.filter(({ filename }) => filename !== names.notarization),
        artifactRecord(names.notarization),
    ];
    releaseEntry.publicationState = publicationState;
    writeJson(artifactPath(names.releaseEntry), releaseEntry);
    const checksummed = [...releaseEntry.artifacts.map(({ filename }) => filename), names.releaseEntry].sort();
    writeFileSync(
        artifactPath(names.checksum),
        `${checksummed.map((filename) => `${sha256File(artifactPath(filename))}  ${filename}`).join('\n')}\n`,
    );
}

function artifactRecord(filename) {
    const path = artifactPath(filename);
    return { filename, byteLength: readFileSync(path).byteLength, sha256: sha256File(path) };
}

function writeSigningRecord({ apple, appleReady }) {
    writeJson(artifactPath(names.signing), {
        schemaVersion: 1,
        product: 'cem-ml',
        commonVersion: version,
        runtimeIdentity: deployment.runtimeIdentity,
        releaseTag: releaseTag(version),
        checksumManifest: {
            filename: names.checksum,
            sha256: sha256File(artifactPath(names.checksum)),
        },
        apple,
        appleNotarization: {
            filename: names.notarization,
            status: appleReady ? 'Accepted' : 'awaiting-release-credentials',
        },
        githubArtifactAttestation: {
            status: 'awaiting-github-oidc',
            bundle: null,
            sha256: null,
        },
        appleReady,
        publicationReady: false,
        mode: appleReady ? 'apple-release-staged' : 'unsigned-local',
    });
}
