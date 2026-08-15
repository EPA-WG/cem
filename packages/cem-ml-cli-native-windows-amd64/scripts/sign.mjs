import { existsSync, readdirSync, rmSync, writeFileSync } from 'node:fs';
import { resolve } from 'node:path';

import { assembleArtifacts, refreshReleaseMetadata } from './assemble.mjs';
import {
    artifactPath,
    assertNativeHost,
    assertUnsignedAuthenticode,
    assertValidAuthenticode,
    assetNames,
    authoritativeVersion,
    buildRoot,
    copyFile,
    deployment,
    outputRoot,
    releaseTag,
    requireFile,
    resetDirectory,
    run,
    sha256File,
    writeJson,
} from './lib.mjs';

assertNativeHost();
const version = authoritativeVersion();
const names = assetNames(version);
const configuration = signingConfiguration();
rmSync(artifactPath(names.attestation), { force: true });

if (configuration === null) {
    const binary = requireFile(resolve(buildRoot, deployment.rustBinary));
    const msi = requireFile(artifactPath(names.msi));
    const binarySignature = assertUnsignedAuthenticode(binary, 'local native executable');
    const msiSignature = assertUnsignedAuthenticode(msi, 'local MSI');
    writeSigningRecord({
        artifactReady: false,
        binary: unsignedRecord(binary, binarySignature),
        mode: 'unsigned-local',
        msi: unsignedRecord(msi, msiSignature),
    });
    if (process.env.CEM_ML_RELEASE_SIGNING === 'required') {
        throw new Error(
            'release signing requires CEM_ML_ARTIFACT_SIGNING_ENDPOINT, ' +
                'CEM_ML_ARTIFACT_SIGNING_ACCOUNT, and CEM_ML_ARTIFACT_SIGNING_PROFILE',
        );
    }
    console.log(`Recorded deterministic unsigned-local signing state for ${deployment.runtimeIdentity} ${version}.`);
} else {
    const signingRoot = resolve(outputRoot, 'signing-work');
    resetDirectory(signingRoot);
    const signedBinary = resolve(signingRoot, deployment.rustBinary);
    copyFile(resolve(buildRoot, deployment.rustBinary), signedBinary);
    const metadataPath = resolve(signingRoot, 'artifact-signing-metadata.json');
    writeFileSync(
        metadataPath,
        `${JSON.stringify({
            Endpoint: configuration.endpoint,
            CodeSigningAccountName: configuration.account,
            CertificateProfileName: configuration.profile,
        })}\n`,
    );

    signFile(signedBinary, metadataPath, configuration);
    const prePackageSignature = assertValidAuthenticode(signedBinary, 'Artifact Signing executable before packaging');
    const assembled = await assembleArtifacts({
        binaryPath: signedBinary,
        distributionMode: 'artifact-signed',
    });
    signFile(artifactPath(names.msi), metadataPath, configuration);
    const msiSignature = assertValidAuthenticode(artifactPath(names.msi), 'Artifact Signing MSI after packaging');
    refreshReleaseMetadata({
        buildMetadata: assembled.packagedBuildMetadata,
        distributionMode: 'artifact-signed',
        installerIdentity: assembled.installerIdentity,
        names,
        version,
    });
    writeSigningRecord({
        artifactReady: true,
        binary: signedRecord(signedBinary, prePackageSignature),
        mode: 'artifact-signing-staged',
        msi: signedRecord(artifactPath(names.msi), msiSignature),
    });
    rmSync(signingRoot, { recursive: true, force: true });
    console.log(`Artifact Signing signed and timestamped ${deployment.runtimeIdentity} ${version} executable and MSI.`);
}

function signingConfiguration() {
    const values = {
        endpoint: process.env.CEM_ML_ARTIFACT_SIGNING_ENDPOINT?.trim(),
        account: process.env.CEM_ML_ARTIFACT_SIGNING_ACCOUNT?.trim(),
        profile: process.env.CEM_ML_ARTIFACT_SIGNING_PROFILE?.trim(),
    };
    const present = Object.values(values).filter((value) => value !== undefined && value.length > 0);
    if (present.length === 0) return null;
    if (present.length !== Object.keys(values).length) {
        throw new Error(
            'Artifact Signing requires endpoint, account, and certificate-profile environment variables together',
        );
    }
    return {
        ...values,
        signtool: findSignTool(),
        dlib: findSigningDlib(),
    };
}

function signFile(path, metadataPath, configuration) {
    run(configuration.signtool, [
        'sign',
        '/v',
        '/debug',
        '/fd',
        'SHA256',
        '/tr',
        'http://timestamp.acs.microsoft.com',
        '/td',
        'SHA256',
        '/dlib',
        configuration.dlib,
        '/dmdf',
        metadataPath,
        path,
    ]);
}

function findSignTool() {
    const supplied = process.env.CEM_ML_SIGNTOOL?.trim();
    if (supplied) return requireFile(supplied, 'CEM_ML_SIGNTOOL');
    const programFilesX86 = process.env['ProgramFiles(x86)'];
    if (!programFilesX86) throw new Error('cannot locate Program Files (x86) for SignTool');
    const sdkBin = resolve(programFilesX86, 'Windows Kits/10/bin');
    const candidates = existsSync(sdkBin)
        ? readdirSync(sdkBin, { withFileTypes: true })
              .filter((entry) => entry.isDirectory())
              .map((entry) => resolve(sdkBin, entry.name, 'x64/signtool.exe'))
              .filter(existsSync)
              .sort()
              .reverse()
        : [];
    if (candidates.length === 0) throw new Error('cannot locate an x64 Windows SDK SignTool');
    return candidates[0];
}

function findSigningDlib() {
    const supplied = process.env.CEM_ML_ARTIFACT_SIGNING_DLIB?.trim();
    if (supplied) return requireFile(supplied, 'CEM_ML_ARTIFACT_SIGNING_DLIB');
    const programFilesX86 = process.env['ProgramFiles(x86)'];
    if (!programFilesX86) throw new Error('cannot locate Program Files (x86) for Artifact Signing');
    const candidates = [
        resolve(programFilesX86, 'Microsoft/ArtifactSigningClientTools/bin/x64/Azure.CodeSigning.Dlib.dll'),
        resolve(programFilesX86, 'Microsoft/ArtifactSigningClientTools/bin/Azure.CodeSigning.Dlib.dll'),
    ];
    const selected = candidates.find(existsSync);
    if (selected === undefined) {
        throw new Error(
            'cannot locate Azure.CodeSigning.Dlib.dll; install Microsoft Artifact Signing Client Tools ' +
                'or set CEM_ML_ARTIFACT_SIGNING_DLIB',
        );
    }
    return selected;
}

function unsignedRecord(path, signature) {
    return {
        status: 'unsigned-local',
        filename: path.split(/[\\/]/).at(-1),
        sha256: sha256File(path),
        authenticodeStatus: signature.status,
        signerSubject: null,
        signerThumbprint: null,
        timeStamperSubject: null,
        timeStamperThumbprint: null,
    };
}

function signedRecord(path, signature) {
    return {
        status: 'artifact-signed',
        filename: path.split(/[\\/]/).at(-1),
        sha256: sha256File(path),
        authenticodeStatus: signature.status,
        signerSubject: signature.signerSubject,
        signerThumbprint: signature.signerThumbprint,
        timeStamperSubject: signature.timeStamperSubject,
        timeStamperThumbprint: signature.timeStamperThumbprint,
    };
}

function writeSigningRecord({ artifactReady, binary, mode, msi }) {
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
        artifactSigning: {
            provider: 'Microsoft Artifact Signing',
            trustModel: 'public-trust',
            timestampAuthority: 'http://timestamp.acs.microsoft.com',
            executable: binary,
            installer: msi,
        },
        githubArtifactAttestation: {
            status: 'awaiting-github-oidc',
            bundle: null,
            sha256: null,
        },
        postDownloadVerification: {
            status: 'awaiting-publication',
            archiveSha256: null,
            installerSha256: null,
            executableAuthenticodeStatus: null,
            installerAuthenticodeStatus: null,
        },
        artifactReady,
        publicationReady: false,
        mode,
    });
}
