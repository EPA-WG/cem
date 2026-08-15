import {
    chmodSync,
    copyFileSync,
    mkdirSync,
    readFileSync,
    renameSync,
    rmSync,
    writeFileSync,
} from 'node:fs';
import { resolve } from 'node:path';

import {
    acquireSyft,
    artifactPath,
    artifactRoot,
    assertNativeHost,
    assetNames,
    authoritativeVersion,
    buildRoot,
    deployment,
    outputRoot,
    projectRoot,
    readJson,
    releaseAssetUrl,
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
    throw new Error('native Linux build metadata drifted from the deployment contract');
}

resetDirectory(artifactRoot);
const workRoot = resolve(outputRoot, 'package-work');
resetDirectory(workRoot);
const archiveParent = resolve(workRoot, 'archive');
const archiveRoot = resolve(archiveParent, names.base);
mkdirSync(resolve(archiveRoot, 'bin'), { recursive: true });
mkdirSync(resolve(archiveRoot, 'share/cem-ml'), { recursive: true });
copyExecutable(resolve(buildRoot, deployment.rustBinary), resolve(archiveRoot, 'bin/cem-ml'));
copyFileSync(resolve(buildRoot, 'capabilities.json'), resolve(archiveRoot, 'share/cem-ml/capabilities.json'));
copyFileSync(resolve(buildRoot, 'build-metadata.json'), resolve(archiveRoot, 'share/cem-ml/build-metadata.json'));
copyFileSync(resolve(workspaceRoot, 'LICENSE'), resolve(archiveRoot, 'LICENSE'));
copyFileSync(resolve(projectRoot, 'README.md'), resolve(archiveRoot, 'README.md'));
setTreeTimestamp(archiveRoot, epoch);

const tarPath = artifactPath(names.archive.slice(0, -3));
run('tar', [
    '--sort=name',
    '--format=ustar',
    '--owner=0',
    '--group=0',
    '--numeric-owner',
    `--mtime=@${epoch}`,
    '-C',
    archiveParent,
    '-cf',
    tarPath,
    names.base,
]);
run('gzip', ['-n', '-9', '-f', tarPath]);
const compressedTar = `${tarPath}.gz`;
if (compressedTar !== artifactPath(names.archive)) {
    renameSync(compressedTar, artifactPath(names.archive));
}

const debRoot = resolve(workRoot, 'deb');
mkdirSync(resolve(debRoot, 'DEBIAN'), { recursive: true });
mkdirSync(resolve(debRoot, 'usr/bin'), { recursive: true });
mkdirSync(resolve(debRoot, 'usr/share/cem-ml'), { recursive: true });
mkdirSync(resolve(debRoot, 'usr/share/doc/cem-ml'), { recursive: true });
copyExecutable(resolve(buildRoot, deployment.rustBinary), resolve(debRoot, 'usr/bin/cem-ml'));
copyFileSync(resolve(buildRoot, 'capabilities.json'), resolve(debRoot, 'usr/share/cem-ml/capabilities.json'));
copyFileSync(resolve(buildRoot, 'build-metadata.json'), resolve(debRoot, 'usr/share/cem-ml/build-metadata.json'));
copyFileSync(resolve(workspaceRoot, 'LICENSE'), resolve(debRoot, 'usr/share/doc/cem-ml/copyright'));
writeFileSync(
    resolve(debRoot, 'DEBIAN/control'),
    [
        `Package: ${deployment.debian.package}`,
        `Version: ${version}`,
        'Section: utils',
        'Priority: optional',
        `Architecture: ${deployment.debian.architecture}`,
        'Maintainer: EPA-WG <https://github.com/EPA-WG>',
        'Homepage: https://github.com/EPA-WG/cem',
        'Description: CEM schema-defined parser, validator, query, and transformation CLI',
        ' Native Linux AMD64 deployment of the common CEM-ML engine.',
        '',
    ].join('\n'),
);
setTreeTimestamp(debRoot, epoch);
run('dpkg-deb', ['--root-owner-group', '--build', debRoot, artifactPath(names.deb)], {
    env: { SOURCE_DATE_EPOCH: String(epoch) },
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
normalizeSbom(artifactPath(names.sbom), version, epoch, buildMetadata.binarySha256);

const primarySubjects = [names.archive, names.deb, names.capability, names.sbom].map(artifactRecord);
writeJson(artifactPath(names.provenance), {
    schemaVersion: 1,
    predicateType: 'https://slsa.dev/provenance/v1',
    status: 'unsigned-build-record',
    builder: {
        id: `nx:${deployment.nxProject}:package`,
        runner: deployment.host.runner,
    },
    buildDefinition: {
        buildType: 'https://cem.dev/build/native-cli/cargo-v1',
        externalParameters: {
            cargoLocked: true,
            cargoRelease: true,
            rustTarget: deployment.rustTarget,
            runtimeIdentity: deployment.runtimeIdentity,
        },
        resolvedDependencies: [
            { uri: 'git+https://github.com/EPA-WG/cem.git', digest: { gitCommit: sourceCommit() } },
            { uri: 'file:Cargo.lock', digest: { sha256: sha256File(resolve(workspaceRoot, 'Cargo.lock')) } },
        ],
    },
    runDetails: {
        metadata: { sourceDateEpoch: epoch },
        byproducts: [{ name: 'syft', version: deployment.syft.version }],
    },
    subject: primarySubjects,
});

writeJson(artifactPath(names.apt), {
    schemaVersion: 1,
    channel: 'apt',
    repository: deployment.debian.repository,
    package: deployment.debian.package,
    version,
    architecture: deployment.debian.architecture,
    immutableSource: {
        releaseTag: releaseTag(version),
        filename: names.deb,
        url: releaseAssetUrl(names.deb, version),
        sha256: sha256File(artifactPath(names.deb)),
    },
    releaseMetadata: {
        generator: 'reprepro',
        signature: 'EPA-WG release GPG identity',
        rebuildExecutable: false,
    },
});

const releaseArtifacts = [
    names.archive,
    names.deb,
    names.capability,
    names.sbom,
    names.provenance,
    names.apt,
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
    capabilityManifestDigest: buildMetadata.capabilitySha256,
    artifacts: releaseArtifacts,
    checksumManifest: names.checksum,
    signingRecord: names.signing,
    publicationState: 'staged-local',
});

const checksummed = [...releaseArtifacts.map(({ filename }) => filename), names.releaseEntry].sort();
writeFileSync(
    artifactPath(names.checksum),
    `${checksummed.map((filename) => `${sha256File(artifactPath(filename))}  ${filename}`).join('\n')}\n`,
);
rmSync(workRoot, { recursive: true, force: true });
console.log(
    `Packaged ${deployment.runtimeIdentity} ${version}: ${names.archive}, ${names.deb}, SPDX SBOM, and release metadata.`,
);

function artifactRecord(filename) {
    const path = artifactPath(filename);
    return { filename, byteLength: readFileSync(path).byteLength, sha256: sha256File(path) };
}

function copyExecutable(source, destination) {
    copyFileSync(requireFile(source), destination);
    chmodSync(destination, 0o755);
}

function normalizeSbom(path, commonVersion, sourceDateEpoch, binarySha256) {
    const sbom = readJson(path);
    if (sbom.spdxVersion !== 'SPDX-2.3') {
        throw new Error(`Syft emitted unsupported SPDX version ${sbom.spdxVersion}`);
    }
    sbom.name = `cem-ml-${deployment.runtimeIdentity}-${commonVersion}`;
    sbom.documentNamespace = `https://cem.dev/spdx/cem-ml/${commonVersion}/${deployment.runtimeIdentity}/${binarySha256}`;
    sbom.creationInfo ??= {};
    sbom.creationInfo.created = new Date(sourceDateEpoch * 1000).toISOString().replace('.000Z', 'Z');
    writeJson(path, sbom);
}
