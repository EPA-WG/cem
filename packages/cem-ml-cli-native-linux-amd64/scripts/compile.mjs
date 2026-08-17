import { chmodSync, copyFileSync } from 'node:fs';
import { resolve } from 'node:path';

import {
    assertNativeHost,
    authoritativeVersion,
    cargoPackageVersion,
    cargoTargetRoot,
    capture,
    compileRoot,
    deployment,
    readJson,
    requireFile,
    resetDirectory,
    run,
    sha256File,
    workspaceRoot,
    writeJson,
} from './lib.mjs';

assertNativeHost();
const version = authoritativeVersion();
const cliVersion = cargoPackageVersion(resolve(workspaceRoot, 'packages/cem_ml_cli/Cargo.toml'));
if (cliVersion !== version) {
    throw new Error(`cem_ml_cli version ${cliVersion} does not match authoritative ${version}`);
}

run('cargo', [
    'build',
    '--locked',
    '--release',
    '--package',
    deployment.rustPackage,
    '--target',
    deployment.rustTarget,
    '--target-dir',
    cargoTargetRoot,
]);

const abiIdentity = `cem-ml-native-cli-v1:${deployment.rustTarget}`;
const capability = JSON.parse(
    capture('cargo', [
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
    ]),
);
if (capability.commonVersion !== version || capability.runtime !== 'native') {
    throw new Error('native capability projection does not match the deployment identity');
}
if (capability.targetIdentity !== deployment.rustTarget || capability.abiIdentity !== abiIdentity) {
    throw new Error('native capability target/ABI projection drifted');
}

resetDirectory(compileRoot);
const sourceBinary = requireFile(
    resolve(cargoTargetRoot, deployment.rustTarget, 'release', deployment.rustBinary),
    'release native binary',
);
const binary = resolve(compileRoot, deployment.rustBinary);
copyFileSync(sourceBinary, binary);
chmodSync(binary, 0o755);
writeJson(resolve(compileRoot, 'capabilities.json'), capability);

const versionOutput = capture(binary, ['version']).trim().split('\n')[0];
if (versionOutput !== `cem-ml ${version}`) {
    throw new Error(`native binary reported ${JSON.stringify(versionOutput)}, expected cem-ml ${version}`);
}
writeJson(resolve(compileRoot, 'compile-metadata.json'), {
    schemaVersion: 1,
    product: 'cem-ml',
    commonVersion: version,
    rustTarget: deployment.rustTarget,
    runtimeIdentity: deployment.runtimeIdentity,
    abiIdentity,
    binary: deployment.rustBinary,
    binarySha256: sha256File(binary),
    capabilitySha256: sha256File(resolve(compileRoot, 'capabilities.json')),
    rustc: capture('rustc', ['--version']).trim(),
    cargo: capture('cargo', ['--version']).trim(),
});

const metadata = readJson(resolve(compileRoot, 'compile-metadata.json'));
console.log(
    `Compiled ${deployment.runtimeIdentity} ${metadata.commonVersion} (${metadata.binarySha256.slice(0, 12)}).`,
);
