import assert from 'node:assert/strict';
import { chmodSync, existsSync, mkdirSync, mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { resolve, sep } from 'node:path';

import {
    artifactPath,
    assetNames,
    authoritativeVersion,
    capture,
    deployment,
    projectRoot,
    run,
} from './lib.mjs';

const mode = process.argv[2];
if (!['install', 'upgrade', 'uninstall'].includes(mode)) {
    throw new Error('usage: smoke.mjs install|upgrade|uninstall');
}
const version = authoritativeVersion();
const names = assetNames(version);
const deb = artifactPath(names.deb);
const installationRoot = mkdtempSync(resolve(tmpdir(), `cem-ml-native-linux-${mode}-`));

try {
    if (mode === 'upgrade') installFixturePredecessor(installationRoot);
    run('dpkg-deb', ['--extract', deb, installationRoot]);
    const binary = resolve(installationRoot, 'usr/bin/cem-ml');
    assert.equal(capture(binary, ['version']).trim().split('\n')[0], `cem-ml ${version}`);
    verifyFunctionalCommand(binary);

    if (mode === 'uninstall') {
        rmSync(resolve(installationRoot, 'usr/bin/cem-ml'));
        rmSync(resolve(installationRoot, 'usr/share/cem-ml'), { recursive: true });
        rmSync(resolve(installationRoot, 'usr/share/doc/cem-ml'), { recursive: true });
        assert.equal(existsSync(binary), false);
        assert.equal(existsSync(resolve(installationRoot, 'usr/share/cem-ml')), false);
    }
    console.log(
        `${deployment.runtimeIdentity} ${mode} smoke passed for cem-ml ${version} in an isolated package root.`,
    );
} finally {
    assert.ok(installationRoot.startsWith(`${tmpdir()}${sep}cem-ml-native-linux-${mode}-`));
    rmSync(installationRoot, { recursive: true, force: true });
}

function installFixturePredecessor(root) {
    const binary = resolve(root, 'usr/bin/cem-ml');
    mkdirSync(resolve(root, 'usr/bin'), { recursive: true });
    mkdirSync(resolve(root, 'usr/share/cem-ml'), { recursive: true });
    writeFileSync(binary, '#!/bin/sh\necho "cem-ml 0.0.0-fixture"\n');
    chmodSync(binary, 0o755);
    writeFileSync(resolve(root, 'usr/share/cem-ml/build-metadata.json'), '{"commonVersion":"0.0.0-fixture"}\n');
    assert.equal(capture(binary, ['version']).trim(), 'cem-ml 0.0.0-fixture');
}

function verifyFunctionalCommand(binary) {
    const fixture = resolve(projectRoot, 'tests/smoke-input.cem');
    const validation = JSON.parse(capture(binary, ['validate', fixture, '--format', 'json']));
    assert.equal(validation.summary.inputCount, 1);
    assert.equal(validation.summary.hardViolationCount, 0);
    const conversion = JSON.parse(
        capture(binary, ['convert', fixture, '--to-format', 'dom-json', '--preserve-source-offsets']),
    );
    assert.equal(conversion.kind, 'document');
}
