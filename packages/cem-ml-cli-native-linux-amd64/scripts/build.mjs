import { chmodSync, copyFileSync } from 'node:fs';
import { resolve } from 'node:path';

import {
    assertNativeHost,
    buildRoot,
    capture,
    compileRoot,
    deployment,
    readJson,
    releaseTag,
    requireFile,
    resetDirectory,
    sha256File,
    sourceCommit,
    sourceEpoch,
    writeJson,
} from './lib.mjs';

assertNativeHost();
const abiIdentity = `cem-ml-native-cli-v1:${deployment.rustTarget}`;
const compileMetadata = readJson(requireFile(resolve(compileRoot, 'compile-metadata.json')));
const version = compileMetadata.commonVersion;
if (
    compileMetadata.product !== 'cem-ml' ||
    compileMetadata.rustTarget !== deployment.rustTarget ||
    compileMetadata.runtimeIdentity !== deployment.runtimeIdentity ||
    compileMetadata.abiIdentity !== abiIdentity ||
    compileMetadata.binary !== deployment.rustBinary ||
    version !== deployment.commonVersion
) {
    throw new Error('compiled native identity does not match the deployment identity');
}
const capabilitySource = requireFile(resolve(compileRoot, 'capabilities.json'));
const capability = readJson(capabilitySource);
if (capability.commonVersion !== version || capability.runtime !== 'native') {
    throw new Error('native capability projection does not match the deployment identity');
}
if (capability.targetIdentity !== deployment.rustTarget || capability.abiIdentity !== abiIdentity) {
    throw new Error('native capability target/ABI projection drifted');
}

resetDirectory(buildRoot);
const sourceBinary = requireFile(
    resolve(compileRoot, deployment.rustBinary),
    'compiled native binary',
);
const binary = resolve(buildRoot, deployment.rustBinary);
copyFileSync(sourceBinary, binary);
chmodSync(binary, 0o755);
const capabilityPath = resolve(buildRoot, 'capabilities.json');
copyFileSync(capabilitySource, capabilityPath);

if (
    sha256File(binary) !== compileMetadata.binarySha256 ||
    sha256File(capabilityPath) !== compileMetadata.capabilitySha256
) {
    throw new Error('compiled native outputs do not match their cached metadata');
}

const versionOutput = capture(binary, ['version']).trim().split('\n')[0];
if (versionOutput !== `cem-ml ${version}`) {
    throw new Error(`native binary reported ${JSON.stringify(versionOutput)}, expected cem-ml ${version}`);
}
writeJson(resolve(buildRoot, 'build-metadata.json'), {
    schemaVersion: 1,
    product: 'cem-ml',
    commonVersion: version,
    sourceCommit: sourceCommit(),
    sourceDateEpoch: sourceEpoch(),
    rustTarget: deployment.rustTarget,
    runtimeIdentity: deployment.runtimeIdentity,
    abiIdentity,
    binary: deployment.rustBinary,
    binarySha256: sha256File(binary),
    capabilitySha256: sha256File(capabilityPath),
    releaseTag: releaseTag(version),
    rustc: compileMetadata.rustc,
    cargo: compileMetadata.cargo,
});

const metadata = readJson(resolve(buildRoot, 'build-metadata.json'));
console.log(
    `Built ${deployment.runtimeIdentity} ${metadata.commonVersion} (${metadata.binarySha256.slice(0, 12)}).`,
);
