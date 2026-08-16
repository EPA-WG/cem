import { copyFileSync } from 'node:fs';
import { resolve } from 'node:path';

import {
    assertNativeHost,
    assertPeAmd64,
    assertStaticMsvcRuntime,
    assertUnsignedAuthenticode,
    authoritativeVersion,
    buildRoot,
    cargoPackageVersion,
    cargoTargetRoot,
    capture,
    deployment,
    releaseTag,
    requireFile,
    resetDirectory,
    run,
    sha256File,
    sourceCommit,
    sourceEpoch,
    workspaceRoot,
    writeJson,
} from './lib.mjs';

assertNativeHost();
const version = authoritativeVersion();
const cliVersion = cargoPackageVersion(resolve(workspaceRoot, 'packages/cem_ml_cli/Cargo.toml'));
if (cliVersion !== version) {
    throw new Error(`cem_ml_cli version ${cliVersion} does not match authoritative ${version}`);
}
if (deployment.msvcRuntime !== 'static') {
    throw new Error(`unsupported MSVC runtime linkage ${deployment.msvcRuntime}`);
}
const rustFlags = ['-C', 'target-feature=+crt-static'];
const cargoEnvironment = { CARGO_ENCODED_RUSTFLAGS: rustFlags.join('\u001f') };

run('cargo.exe', [
    'build',
    '--locked',
    '--release',
    '--package',
    deployment.rustPackage,
    '--target',
    deployment.rustTarget,
    '--target-dir',
    cargoTargetRoot,
], { env: cargoEnvironment });

const abiIdentity = `cem-ml-native-cli-v1:${deployment.rustTarget}`;
const capability = JSON.parse(
    capture(
        'cargo.exe',
        [
            'run',
            '--locked',
            '--release',
            '--package',
            deployment.rustPackage,
            '--example',
            'native-capability-emit',
            '--target',
            deployment.rustTarget,
            '--target-dir',
            cargoTargetRoot,
            '--',
            deployment.rustTarget,
            abiIdentity,
        ],
        { env: cargoEnvironment },
    ),
);
if (capability.commonVersion !== version || capability.runtime !== 'native') {
    throw new Error('native capability projection does not match the deployment identity');
}
if (capability.targetIdentity !== deployment.rustTarget || capability.abiIdentity !== abiIdentity) {
    throw new Error('native capability target/ABI projection drifted');
}

resetDirectory(buildRoot);
const sourceBinary = requireFile(
    resolve(cargoTargetRoot, deployment.rustTarget, 'release', deployment.rustBinary),
    'release native binary',
);
const binary = resolve(buildRoot, deployment.rustBinary);
copyFileSync(sourceBinary, binary);
assertPeAmd64(binary);
assertStaticMsvcRuntime(binary, 'freshly built native executable');
assertUnsignedAuthenticode(binary, 'freshly built native executable');
writeJson(resolve(buildRoot, 'capabilities.json'), capability);

const versionOutput = capture(binary, ['version']).trim().split('\n')[0].replace(/\r$/, '');
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
    msvcRuntime: deployment.msvcRuntime,
    rustFlags,
    runtimeIdentity: deployment.runtimeIdentity,
    abiIdentity,
    binary: deployment.rustBinary,
    binarySha256: sha256File(binary),
    capabilitySha256: sha256File(resolve(buildRoot, 'capabilities.json')),
    releaseTag: releaseTag(version),
    authenticode: 'unsigned-local',
    rustc: capture('rustc.exe', ['--version']).trim(),
    cargo: capture('cargo.exe', ['--version']).trim(),
});

console.log(`Built ${deployment.runtimeIdentity} ${version} (${sha256File(binary).slice(0, 12)}).`);
