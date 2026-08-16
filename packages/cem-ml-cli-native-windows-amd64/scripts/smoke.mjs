import assert from 'node:assert/strict';
import { copyFileSync, existsSync, mkdirSync, rmSync } from 'node:fs';
import { resolve } from 'node:path';

import { buildMsi } from './installer.mjs';
import {
    artifactPath,
    assertNativeHost,
    assetNames,
    authoritativeVersion,
    buildRoot,
    capture,
    deployment,
    outputRoot,
    productCode,
    projectRoot,
    readJson,
    resetDirectory,
    runResult,
    setTreeTimestamp,
    sourceEpoch,
    workspaceRoot,
    writeJson,
} from './lib.mjs';

const mode = process.argv[2];
if (!['install', 'upgrade', 'uninstall'].includes(mode)) {
    throw new Error('usage: smoke.mjs install|upgrade|uninstall');
}
assertNativeHost();
const version = authoritativeVersion();
const names = assetNames(version);
const currentMsi = artifactPath(names.msi);
const smokeRoot = resolve(outputRoot, `smoke-${mode}`);
const installRoot = resolve(process.env.ProgramW6432 || process.env.ProgramFiles, 'EPA-WG/CEM-ML');
const binary = resolve(installRoot, 'cem-ml.exe');
const currentCode = productCode(version);
const fixtureVersion = '0.0.0';
const fixtureCode = productCode(fixtureVersion);
const externalCommandTimeoutMs = 120_000;

resetDirectory(smokeRoot);
const fixtureMsi = createFixtureMsi(smokeRoot);
cleanupInstallations();
let failure;
try {
    if (mode === 'upgrade') {
        installMsi(fixtureMsi, 'fixture-install');
        assert.equal(productState(fixtureCode), 5, 'fixture MSI was not installed');
        assert.equal(readJson(resolve(installRoot, 'share/cem-ml/build-metadata.json')).commonVersion, '0.0.0-fixture');
    }

    installMsi(currentMsi, mode === 'upgrade' ? 'current-upgrade' : 'current-install');
    assert.equal(productState(currentCode), 5, 'current MSI was not installed');
    assert.equal(captureSmoke(binary, ['version']).trim().split('\n')[0].replace(/\r$/, ''), `cem-ml ${version}`);
    verifyFunctionalCommand(binary);
    const metadata = readJson(resolve(installRoot, 'share/cem-ml/build-metadata.json'));
    assert.equal(metadata.commonVersion, version);
    assert.equal(metadata.runtimeIdentity, deployment.runtimeIdentity);

    if (mode === 'upgrade') {
        assert.notEqual(productState(fixtureCode), 5, 'fixture MSI survived the major upgrade');
    }
    if (mode === 'uninstall') {
        uninstallProduct(currentCode, 'current-uninstall');
        assert.notEqual(productState(currentCode), 5, 'current MSI remained registered after uninstall');
        assert.equal(existsSync(binary), false);
        assert.equal(existsSync(installRoot), false);
    }
    console.log(`${deployment.runtimeIdentity} MSI ${mode} smoke passed for cem-ml ${version}.`);
} catch (error) {
    failure = error;
}
try {
    cleanupInstallations();
} catch (error) {
    failure =
        failure === undefined
            ? error
            : new AggregateError([failure, error], `${mode} smoke and post-smoke cleanup both failed`);
}
if (failure === undefined) {
    rmSync(smokeRoot, { recursive: true, force: true });
} else {
    console.error(`Preserved failed ${mode} smoke diagnostics at ${smokeRoot}.`);
    throw failure;
}

function createFixtureMsi(root) {
    const payloadRoot = resolve(root, 'fixture-payload');
    mkdirSync(resolve(payloadRoot, 'bin'), { recursive: true });
    mkdirSync(resolve(payloadRoot, 'share/cem-ml'), { recursive: true });
    copyFileSync(resolve(buildRoot, deployment.rustBinary), resolve(payloadRoot, 'bin/cem-ml.exe'));
    writeJson(resolve(payloadRoot, 'share/cem-ml/capabilities.json'), {
        schemaVersion: 1,
        commonVersion: fixtureVersion,
        runtime: 'native-fixture',
    });
    writeJson(resolve(payloadRoot, 'share/cem-ml/build-metadata.json'), {
        schemaVersion: 1,
        product: 'cem-ml',
        commonVersion: '0.0.0-fixture',
        runtimeIdentity: 'native-windows-amd64-fixture',
    });
    copyFileSync(resolve(workspaceRoot, 'LICENSE'), resolve(payloadRoot, 'LICENSE'));
    copyFileSync(resolve(projectRoot, 'README.md'), resolve(payloadRoot, 'README.md'));
    setTreeTimestamp(payloadRoot, sourceEpoch());
    const destination = resolve(root, 'cem-ml-0.0.0-fixture.msi');
    buildMsi({
        destination,
        payloadRoot,
        version: fixtureVersion,
        workRoot: resolve(root, 'fixture-wix'),
        sourceEpoch: sourceEpoch(),
    });
    return destination;
}

function installMsi(path, label) {
    runMsi(['/i', path, '/qn', '/norestart', '/l*v', resolve(smokeRoot, `${label}.log`)]);
}

function uninstallProduct(code, label, cleanup = false) {
    runMsi(['/x', code, '/qn', '/norestart', '/l*v', resolve(smokeRoot, `${label}.log`)], cleanup);
}

function cleanupInstallations() {
    uninstallProduct(currentCode, 'cleanup-current', true);
    uninstallProduct(fixtureCode, 'cleanup-fixture', true);
}

function runMsi(args, cleanup = false) {
    const logIndex = args.findIndex((argument) => argument.toLowerCase() === '/l*v');
    const logPath = logIndex >= 0 ? args[logIndex + 1] : undefined;
    console.log(`Starting msiexec ${args[0]} ${args[1]}; log: ${logPath ?? 'not configured'}.`);
    const result = runResult('msiexec.exe', args, {
        stdio: 'pipe',
        timeout: externalCommandTimeoutMs,
    });
    const accepted = cleanup ? [0, 1605, 1614, 1641, 3010] : [0, 1641, 3010];
    if (!accepted.includes(result.status)) {
        const detail = result.error?.message || result.stderr || result.stdout || `exit ${result.status}`;
        throw new Error(`msiexec ${args.join(' ')} failed: ${detail}`);
    }
    console.log(`Completed msiexec ${args[0]} ${args[1]} with exit ${result.status}.`);
}

function productState(code) {
    const script = [
        '$installer = New-Object -ComObject WindowsInstaller.Installer',
        '$installer.ProductState($env:CEM_ML_PRODUCT_CODE)',
    ].join('\n');
    return Number(
        capture('powershell.exe', ['-NoLogo', '-NoProfile', '-NonInteractive', '-Command', script], {
            env: { CEM_ML_PRODUCT_CODE: code },
            timeout: externalCommandTimeoutMs,
        }).trim(),
    );
}

function verifyFunctionalCommand(executable) {
    const fixture = resolve(projectRoot, 'tests/smoke-input.cem');
    const validation = JSON.parse(captureSmoke(executable, ['validate', fixture, '--format', 'json']));
    assert.equal(validation.summary.inputCount, 1);
    assert.equal(validation.summary.hardViolationCount, 0);
    const conversion = JSON.parse(
        captureSmoke(executable, ['convert', fixture, '--to-format', 'dom-json', '--preserve-source-offsets']),
    );
    assert.equal(conversion.kind, 'document');
}

function captureSmoke(command, args) {
    return capture(command, args, { timeout: externalCommandTimeoutMs });
}
