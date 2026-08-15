import assert from 'node:assert/strict';
import { chmodSync, existsSync, mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { resolve, sep } from 'node:path';
import { pathToFileURL } from 'node:url';

import { renderFormula } from './formula.mjs';
import {
    artifactPath,
    assetNames,
    assertNativeHost,
    authoritativeVersion,
    capture,
    deployment,
    projectRoot,
    run,
    runResult,
    sha256File,
} from './lib.mjs';
import { writeDeterministicTarGz } from './tar.mjs';

const mode = process.argv[2];
if (!['install', 'upgrade', 'uninstall'].includes(mode)) {
    throw new Error('usage: smoke.mjs install|upgrade|uninstall');
}
assertNativeHost();
const version = authoritativeVersion();
const names = assetNames(version);
const archive = artifactPath(names.archive);
const smokeRoot = mkdtempSync(resolve(tmpdir(), `cem-ml-native-macos-${mode}-`));
const tap = 'cemtest/native-cem';
const formula = `${tap}/${deployment.homebrew.formula}`;
const brewEnvironment = {
    HOMEBREW_NO_AUTO_UPDATE: '1',
    HOMEBREW_NO_INSTALL_CLEANUP: '1',
    HOMEBREW_NO_INSTALL_FROM_API: '1',
    XDG_CONFIG_HOME: resolve(smokeRoot, 'xdg'),
};

try {
    cleanupBrew();
    const sourceTap = resolve(smokeRoot, 'homebrew-native-cem');
    const currentFormula = localFormula(archive, version);
    if (mode === 'upgrade') {
        const fixtureArchive = createFixtureArchive();
        prepareTap(sourceTap, localFormula(fixtureArchive, '0.0.0'));
    } else {
        prepareTap(sourceTap, currentFormula);
    }
    brew(['tap', tap, sourceTap]);
    brew(['trust', '--formula', formula]);
    brew(['install', '--build-from-source', formula]);

    let binary = installedBinary();
    if (mode === 'upgrade') {
        assert.equal(captureWithBrew(binary, ['version']).trim(), 'cem-ml 0.0.0-fixture');
        const tappedRepository = captureWithBrew('brew', ['--repository', tap]).trim();
        writeFileSync(resolve(tappedRepository, 'Formula/cem-ml.rb'), currentFormula);
        run('git', ['add', 'Formula/cem-ml.rb'], { cwd: tappedRepository });
        run('git', ['commit', '-m', `upgrade cem-ml fixture to ${version}`], {
            cwd: tappedRepository,
        });
        brew(['upgrade', formula]);
        binary = installedBinary();
    }

    assert.equal(captureWithBrew(binary, ['version']).trim().split('\n')[0], `cem-ml ${version}`);
    verifyFunctionalCommand(binary);
    brew(['test', formula]);

    if (mode === 'uninstall') {
        brew(['uninstall', '--force', formula]);
        assert.equal(existsSync(binary), false);
        assert.notEqual(brewResult(['list', '--versions', formula]).status, 0);
    }
    console.log(`${deployment.runtimeIdentity} Homebrew ${mode} smoke passed for cem-ml ${version}.`);
} finally {
    cleanupBrew();
    assert.ok(smokeRoot.startsWith(`${tmpdir()}${sep}cem-ml-native-macos-${mode}-`));
    rmSync(smokeRoot, { recursive: true, force: true });
}

function brew(args) {
    return run('brew', args, { env: brewEnvironment });
}

function brewResult(args) {
    return runResult('brew', args, { env: brewEnvironment, stdio: 'pipe' });
}

function captureWithBrew(command, args) {
    return capture(command, args, { env: brewEnvironment });
}

function cleanupBrew() {
    brewResult(['uninstall', '--force', formula]);
    brewResult(['untrust', '--formula', formula]);
    brewResult(['untap', '--force', tap]);
}

function installedBinary() {
    const prefix = captureWithBrew('brew', ['--prefix', formula]).trim();
    return resolve(prefix, 'bin/cem-ml');
}

function localFormula(path, formulaVersion) {
    return renderFormula({
        archiveSha256: sha256File(path),
        archiveUrl: pathToFileURL(path).href,
        version: formulaVersion,
    });
}

function prepareTap(root, formulaText) {
    mkdirSync(resolve(root, 'Formula'), { recursive: true });
    writeFileSync(resolve(root, 'Formula/cem-ml.rb'), formulaText);
    run('git', ['init'], { cwd: root });
    run('git', ['symbolic-ref', 'HEAD', 'refs/heads/main'], { cwd: root });
    run('git', ['config', 'user.name', 'CEM native smoke'], { cwd: root });
    run('git', ['config', 'user.email', 'native-smoke@invalid.example'], { cwd: root });
    run('git', ['add', 'Formula/cem-ml.rb'], { cwd: root });
    run('git', ['commit', '-m', 'add fixture formula'], { cwd: root });
}

function createFixtureArchive() {
    const fixtureRoot = resolve(smokeRoot, 'fixture-package');
    const fixtureBinary = resolve(fixtureRoot, 'bin/cem-ml');
    mkdirSync(resolve(fixtureRoot, 'bin'), { recursive: true });
    mkdirSync(resolve(fixtureRoot, 'share/cem-ml'), { recursive: true });
    writeFileSync(fixtureBinary, '#!/bin/sh\necho "cem-ml 0.0.0-fixture"\n');
    chmodSync(fixtureBinary, 0o755);
    writeFileSync(resolve(fixtureRoot, 'share/cem-ml/capabilities.json'), '{}\n');
    writeFileSync(resolve(fixtureRoot, 'share/cem-ml/build-metadata.json'), '{"commonVersion":"0.0.0-fixture"}\n');
    const fixtureArchive = resolve(smokeRoot, 'cem-ml-0.0.0-macos-arm64.tar.gz');
    writeDeterministicTarGz(fixtureArchive, fixtureRoot, 'cem-ml-0.0.0-macos-arm64', 1);
    return fixtureArchive;
}

function verifyFunctionalCommand(binary) {
    const fixture = resolve(projectRoot, 'tests/smoke-input.cem');
    const validation = JSON.parse(captureWithBrew(binary, ['validate', fixture, '--format', 'json']));
    assert.equal(validation.summary.inputCount, 1);
    assert.equal(validation.summary.hardViolationCount, 0);
    const conversion = JSON.parse(
        captureWithBrew(binary, ['convert', fixture, '--to-format', 'dom-json', '--preserve-source-offsets']),
    );
    assert.equal(conversion.kind, 'document');
}
