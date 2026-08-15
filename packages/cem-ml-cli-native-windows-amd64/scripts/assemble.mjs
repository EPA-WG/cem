import { copyFileSync, mkdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { resolve } from 'node:path';

import { buildMsi } from './installer.mjs';
import {
    acquireSyft,
    artifactPath,
    artifactRoot,
    assertNativeHost,
    assertUnsignedAuthenticode,
    assertValidAuthenticode,
    assetNames,
    authoritativeVersion,
    buildRoot,
    deployment,
    outputRoot,
    projectRoot,
    readJson,
    releaseTag,
    requireFile,
    resetDirectory,
    run,
    setTreeTimestamp,
    sha256File,
    sourceCommit,
    sourceEpoch,
    workspaceRoot,
    writeJson,
} from './lib.mjs';
import { writeWingetArtifacts } from './winget.mjs';
import { writeDeterministicZip } from './zip.mjs';

export async function assembleArtifacts({
    binaryPath = resolve(buildRoot, deployment.rustBinary),
    distributionMode = 'unsigned-local',
} = {}) {
    assertNativeHost();
    const version = authoritativeVersion();
    const names = assetNames(version);
    const epoch = sourceEpoch();
    const buildMetadata = readJson(requireFile(resolve(buildRoot, 'build-metadata.json')));
    if (
        buildMetadata.commonVersion !== version ||
        buildMetadata.rustTarget !== deployment.rustTarget ||
        buildMetadata.runtimeIdentity !== deployment.runtimeIdentity
    ) {
        throw new Error('native Windows build metadata drifted from the deployment contract');
    }
    const authenticode =
        distributionMode === 'artifact-signed'
            ? assertValidAuthenticode(binaryPath, 'pre-package native executable')
            : assertUnsignedAuthenticode(binaryPath, 'pre-package native executable');

    resetDirectory(artifactRoot);
    const workRoot = resolve(outputRoot, 'package-work');
    resetDirectory(workRoot);
    const archiveRoot = resolve(workRoot, names.base);
    mkdirSync(resolve(archiveRoot, 'bin'), { recursive: true });
    mkdirSync(resolve(archiveRoot, 'share/cem-ml'), { recursive: true });
    copyFileSync(requireFile(binaryPath, 'package binary'), resolve(archiveRoot, 'bin/cem-ml.exe'));
    copyFileSync(resolve(buildRoot, 'capabilities.json'), resolve(archiveRoot, 'share/cem-ml/capabilities.json'));
    const packagedBuildMetadata = {
        ...buildMetadata,
        binarySha256: sha256File(binaryPath),
        authenticode: distributionMode,
        authenticodeSigner: authenticode.signerSubject ?? null,
        authenticodeTimestampAuthority: authenticode.timeStamperSubject ?? null,
        distributionMode,
    };
    writeJson(resolve(archiveRoot, 'share/cem-ml/build-metadata.json'), packagedBuildMetadata);
    copyFileSync(resolve(workspaceRoot, 'LICENSE'), resolve(archiveRoot, 'LICENSE'));
    copyFileSync(resolve(projectRoot, 'README.md'), resolve(archiveRoot, 'README.md'));
    setTreeTimestamp(archiveRoot, epoch);

    writeDeterministicZip(artifactPath(names.archive), archiveRoot, names.base, epoch);
    const installerIdentity = buildMsi({
        destination: artifactPath(names.msi),
        payloadRoot: archiveRoot,
        version,
        workRoot,
        sourceEpoch: epoch,
    });
    copyFileSync(resolve(buildRoot, 'capabilities.json'), artifactPath(names.capability));
    const syft = await acquireSyft();
    run(
        syft,
        [
            'scan',
            `dir:${archiveRoot}`,
            '--source-name',
            `cem-ml-${deployment.runtimeIdentity}`,
            '--source-version',
            version,
            '--output',
            `spdx-json=${artifactPath(names.sbom)}`,
        ],
        { env: { SYFT_CHECK_FOR_APP_UPDATE: 'false' } },
    );
    normalizeSbom(artifactPath(names.sbom), version, epoch, packagedBuildMetadata.binarySha256);
    refreshReleaseMetadata({
        buildMetadata: packagedBuildMetadata,
        distributionMode,
        installerIdentity,
        names,
        version,
    });
    rmSync(workRoot, { recursive: true, force: true });
    return { installerIdentity, names, packagedBuildMetadata, version };
}

export function refreshReleaseMetadata({ buildMetadata, distributionMode, installerIdentity, names, version }) {
    writeWingetArtifacts({ names, version });
    const primarySubjects = [names.archive, names.msi, names.capability, names.sbom].map(artifactRecord);
    writeJson(artifactPath(names.provenance), {
        schemaVersion: 1,
        predicateType: 'https://slsa.dev/provenance/v1',
        status: `${distributionMode}-build-record`,
        builder: {
            id: `nx:${deployment.nxProject}:package`,
            runner: deployment.host.runner,
        },
        buildDefinition: {
            buildType: 'https://cem.dev/build/native-cli/cargo-wix-v1',
            externalParameters: {
                cargoLocked: true,
                cargoRelease: true,
                rustTarget: deployment.rustTarget,
                runtimeIdentity: deployment.runtimeIdentity,
                wixVersion: deployment.wix.version,
                productCode: installerIdentity.productCode,
                packageCode: installerIdentity.packageCode,
                upgradeCode: installerIdentity.upgradeCode,
                distributionMode,
            },
            resolvedDependencies: [
                {
                    uri: 'git+https://github.com/EPA-WG/cem.git',
                    digest: { gitCommit: sourceCommit() },
                },
                {
                    uri: 'file:Cargo.lock',
                    digest: { sha256: sha256File(resolve(workspaceRoot, 'Cargo.lock')) },
                },
            ],
        },
        runDetails: {
            metadata: { sourceDateEpoch: sourceEpoch() },
            byproducts: [
                { name: 'syft', version: deployment.syft.version },
                { name: 'wix', version: deployment.wix.version },
            ],
        },
        subject: primarySubjects,
    });

    const releaseArtifacts = [
        names.archive,
        names.msi,
        names.capability,
        names.sbom,
        names.provenance,
        names.winget,
        names.wingetInstaller,
        names.wingetLocale,
        names.wingetVersion,
    ].map(artifactRecord);
    writeJson(artifactPath(names.releaseEntry), {
        schemaVersion: 1,
        product: 'cem-ml',
        commonVersion: version,
        sourceCommit: sourceCommit(),
        releaseTag: releaseTag(version),
        runtimeIdentity: deployment.runtimeIdentity,
        targetIdentity: deployment.rustTarget,
        abiIdentity: buildMetadata.abiIdentity,
        capabilityManifestDigest: sha256File(artifactPath(names.capability)),
        installerIdentity,
        artifacts: releaseArtifacts,
        checksumManifest: names.checksum,
        signingRecord: names.signing,
        publicationState: distributionMode === 'artifact-signed' ? 'signed-staged' : 'staged-local',
    });

    const checksummed = [...releaseArtifacts.map(({ filename }) => filename), names.releaseEntry].sort();
    writeFileSync(
        artifactPath(names.checksum),
        `${checksummed.map((filename) => `${sha256File(artifactPath(filename))}  ${filename}`).join('\n')}\n`,
    );
}

function artifactRecord(filename) {
    const path = artifactPath(filename);
    return { filename, byteLength: readFileSync(path).byteLength, sha256: sha256File(path) };
}

function normalizeSbom(path, commonVersion, sourceDateEpoch, binarySha256) {
    const sbom = readJson(path);
    if (sbom.spdxVersion !== 'SPDX-2.3') {
        throw new Error(`Syft emitted unsupported SPDX version ${sbom.spdxVersion}`);
    }
    sbom.name = `cem-ml-${deployment.runtimeIdentity}-${commonVersion}`;
    sbom.documentNamespace =
        `https://cem.dev/spdx/cem-ml/${commonVersion}/` + `${deployment.runtimeIdentity}/${binarySha256}`;
    sbom.creationInfo ??= {};
    sbom.creationInfo.created = new Date(sourceDateEpoch * 1000).toISOString().replace('.000Z', 'Z');
    writeJson(path, sbom);
}
