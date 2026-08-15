import assert from 'node:assert/strict';
import { copyFileSync, mkdirSync, rmSync, writeFileSync } from 'node:fs';
import { resolve } from 'node:path';

import { buildMsi } from './installer.mjs';
import {
    artifactPath,
    assertNativeHost,
    assetNames,
    authoritativeVersion,
    capture,
    deployment,
    outputRoot,
    productCode,
    projectRoot,
    readJson,
    requireFile,
    resetDirectory,
    runResult,
    setTreeTimestamp,
    sourceEpoch,
    writeJson,
} from './lib.mjs';

assertNativeHost();
const version = authoritativeVersion();
const names = assetNames(version);
const sandboxRoot = resolve(outputRoot, 'sandbox-work');
const inputRoot = resolve(sandboxRoot, 'input');
const resultRoot = resolve(sandboxRoot, 'result');
resetDirectory(sandboxRoot);
mkdirSync(inputRoot, { recursive: true });
mkdirSync(resultRoot, { recursive: true });

const featureState = capture('powershell.exe', [
    '-NoLogo',
    '-NoProfile',
    '-NonInteractive',
    '-Command',
    '(Get-WindowsOptionalFeature -Online -FeatureName Containers-DisposableClientVM).State.ToString()',
]).trim();
if (featureState !== 'Enabled') {
    throw new Error(`Windows Sandbox optional feature is ${featureState}, expected Enabled`);
}

const extractedRoot = resolve(sandboxRoot, 'archive');
expandArchive(artifactPath(names.archive), extractedRoot);
const currentPayload = resolve(extractedRoot, names.base);
const fixturePayload = resolve(sandboxRoot, 'fixture-payload');
mkdirSync(resolve(fixturePayload, 'bin'), { recursive: true });
mkdirSync(resolve(fixturePayload, 'share/cem-ml'), { recursive: true });
copyFileSync(resolve(currentPayload, 'bin/cem-ml.exe'), resolve(fixturePayload, 'bin/cem-ml.exe'));
writeJson(resolve(fixturePayload, 'share/cem-ml/capabilities.json'), {
    schemaVersion: 1,
    commonVersion: '0.0.0',
    runtime: 'native-fixture',
});
writeJson(resolve(fixturePayload, 'share/cem-ml/build-metadata.json'), {
    schemaVersion: 1,
    product: 'cem-ml',
    commonVersion: '0.0.0-fixture',
    runtimeIdentity: 'native-windows-amd64-fixture',
});
copyFileSync(resolve(currentPayload, 'LICENSE'), resolve(fixturePayload, 'LICENSE'));
copyFileSync(resolve(currentPayload, 'README.md'), resolve(fixturePayload, 'README.md'));
setTreeTimestamp(fixturePayload, sourceEpoch());

const fixtureMsi = resolve(inputRoot, 'fixture.msi');
buildMsi({
    destination: fixtureMsi,
    payloadRoot: fixturePayload,
    version: '0.0.0',
    workRoot: resolve(sandboxRoot, 'fixture-wix'),
    sourceEpoch: sourceEpoch(),
});
copyFileSync(artifactPath(names.msi), resolve(inputRoot, 'current.msi'));
copyFileSync(resolve(projectRoot, 'tests/smoke-input.cem'), resolve(inputRoot, 'smoke-input.cem'));
copyFileSync(resolve(projectRoot, 'scripts/sandbox-lifecycle.ps1'), resolve(inputRoot, 'sandbox-lifecycle.ps1'));

const command = [
    'powershell.exe -NoLogo -NoProfile -ExecutionPolicy Bypass',
    '-File C:\\CemMlSmoke\\sandbox-lifecycle.ps1',
    '-InputRoot C:\\CemMlSmoke',
    '-OutputRoot C:\\CemMlResult',
    `-CurrentProductCode '${productCode(version)}'`,
    `-FixtureProductCode '${productCode('0.0.0')}'`,
    `-ExpectedVersion '${version}'`,
].join(' ');
const configuration = resolve(sandboxRoot, 'cem-ml-smoke.wsb');
writeFileSync(
    configuration,
    `<Configuration>
  <VGpu>Disable</VGpu>
  <Networking>Disable</Networking>
  <ClipboardRedirection>Disable</ClipboardRedirection>
  <PrinterRedirection>Disable</PrinterRedirection>
  <MemoryInMB>4096</MemoryInMB>
  <MappedFolders>
    <MappedFolder>
      <HostFolder>${escapeXml(inputRoot)}</HostFolder>
      <SandboxFolder>C:\\CemMlSmoke</SandboxFolder>
      <ReadOnly>true</ReadOnly>
    </MappedFolder>
    <MappedFolder>
      <HostFolder>${escapeXml(resultRoot)}</HostFolder>
      <SandboxFolder>C:\\CemMlResult</SandboxFolder>
      <ReadOnly>false</ReadOnly>
    </MappedFolder>
  </MappedFolders>
  <LogonCommand>
    <Command>${escapeXml(command)}</Command>
  </LogonCommand>
</Configuration>
`,
);

const sandboxExecutable = requireFile(
    resolve(process.env.WINDIR || 'C:/Windows', 'System32/WindowsSandbox.exe'),
    'Windows Sandbox executable',
);
const timeout = Number(process.env.CEM_ML_WINDOWS_SANDBOX_TIMEOUT_MS || 1_500_000);
const result = runResult(sandboxExecutable, [configuration], { stdio: 'pipe', timeout });
if (result.error?.code === 'ETIMEDOUT') {
    throw new Error(`Windows Sandbox lifecycle exceeded ${timeout}ms`);
}
if (result.status !== 0) {
    throw new Error(
        `Windows Sandbox exited ${result.status}: ${result.stderr || result.stdout || result.error?.message}`,
    );
}
const lifecycle = readJson(requireFile(resolve(resultRoot, 'result.json'), 'Sandbox lifecycle result'));
assert.equal(lifecycle.status, 'passed', lifecycle.error || 'Windows Sandbox lifecycle failed');
assert.deepEqual(new Set(lifecycle.completed), new Set(['install', 'upgrade', 'uninstall']));
console.log(
    `${deployment.runtimeIdentity} Windows Sandbox install/upgrade/uninstall smoke passed for cem-ml ${version}.`,
);
rmSync(sandboxRoot, { recursive: true, force: true });

function expandArchive(archive, destination) {
    const result = runResult(
        'pwsh.exe',
        [
            '-NoLogo',
            '-NoProfile',
            '-NonInteractive',
            '-Command',
            'Expand-Archive -LiteralPath $env:CEM_ML_ARCHIVE_PATH -DestinationPath $env:CEM_ML_ARCHIVE_DESTINATION -Force',
        ],
        {
            env: {
                CEM_ML_ARCHIVE_PATH: archive,
                CEM_ML_ARCHIVE_DESTINATION: destination,
            },
        },
    );
    if (result.status !== 0) {
        throw new Error(`Expand-Archive failed: ${result.stderr || result.stdout || result.error?.message}`);
    }
}

function escapeXml(value) {
    return value.replaceAll('&', '&amp;').replaceAll('"', '&quot;').replaceAll('<', '&lt;').replaceAll('>', '&gt;');
}
