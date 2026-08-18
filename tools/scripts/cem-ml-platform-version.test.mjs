import assert from 'node:assert/strict';
import { cpSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { dirname, resolve } from 'node:path';
import test from 'node:test';

import { governedPlatformVersionFiles, synchronizePlatformVersion } from './cem-ml-platform-version.mjs';

const workspaceRoot = resolve(import.meta.dirname, '../..');

test('Cargo authority synchronizes every platform manifest and exact dependency without check-mode writes', () => {
    const fixtureRoot = mkdtempSync(resolve(tmpdir(), 'cem-ml-platform-version-'));
    try {
        for (const relativePath of governedPlatformVersionFiles) {
            const destination = resolve(fixtureRoot, relativePath);
            mkdirSync(dirname(destination), { recursive: true });
            cpSync(resolve(workspaceRoot, relativePath), destination);
        }
        const authorityPath = resolve(fixtureRoot, 'packages/cem_ml/Cargo.toml');
        writeFileSync(
            authorityPath,
            readFileSync(authorityPath, 'utf8').replace(/^version\s*=\s*"[^"]+"\s*$/m, 'version = "2.3.4"'),
        );

        const synchronized = synchronizePlatformVersion({ workspaceRoot: fixtureRoot, write: true });
        assert.equal(synchronized.version, '2.3.4');
        assert.ok(synchronized.changedFiles.length >= 10);
        const cliCargo = readFileSync(resolve(fixtureRoot, 'packages/cem_ml_cli/Cargo.toml'), 'utf8');
        assert.match(cliCargo, /^version = "2\.3\.4"$/m);
        assert.match(cliCargo, /cem-ml = \{ version = "=2\.3\.4", path = "\.\.\/cem_ml"/);
        assert.match(cliCargo, /cem-ml-transform-cem-ql = \{ version = "=2\.3\.4"/);
        const npmCli = JSON.parse(readFileSync(resolve(fixtureRoot, 'packages/cem-ml-cli-npm/package.json')));
        assert.equal(npmCli.version, '2.3.4');
        assert.equal(npmCli.dependencies['@epa-wg/cem-ml'], '2.3.4');
        for (const relativePath of [
            'packages/cem-ml-cli-native-linux-amd64/deployment.json',
            'packages/cem-ml-cli-native-brew-arm64/deployment.json',
            'packages/cem-ml-cli-native-windows-amd64/deployment.json',
        ]) {
            assert.equal(JSON.parse(readFileSync(resolve(fixtureRoot, relativePath))).commonVersion, '2.3.4');
        }
        synchronizePlatformVersion({ workspaceRoot: fixtureRoot, write: false });

        const driftPath = resolve(fixtureRoot, 'packages/cem-ml-cli-npm/package.json');
        const drifted = readFileSync(driftPath, 'utf8').replace(
            '"@epa-wg/cem-ml": "2.3.4"',
            '"@epa-wg/cem-ml": "9.9.9"',
        );
        writeFileSync(driftPath, drifted);
        const beforeCheck = readFileSync(driftPath, 'utf8');
        assert.throws(
            () => synchronizePlatformVersion({ workspaceRoot: fixtureRoot, write: false }),
            /platform version drift/,
        );
        assert.equal(readFileSync(driftPath, 'utf8'), beforeCheck, 'read-only drift verification mutated its input');
    } finally {
        rmSync(fixtureRoot, { recursive: true, force: true });
    }
});
