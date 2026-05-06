import { describe, it } from 'node:test';
import assert from 'node:assert/strict';
import { mkdtemp, writeFile } from 'node:fs/promises';
import { join } from 'node:path';
import { tmpdir } from 'node:os';
import { runCemDomCli } from './cli.ts';

describe('runCemDomCli', () => {
    it('prints help', async () => {
        const result = await runCemDomCli(['help']);

        assert.equal(result.exitCode, 0);
        assert.match(result.stdout, /cem-dom <command>/);
    });

    it('parses and validates a file', async () => {
        const dir = await mkdtemp(join(tmpdir(), 'cem-dom-'));
        const file = join(dir, 'fixture.html');
        await writeFile(
            file,
            '<main data-cem-screen="login" aria-labelledby="title"><h1 id="title">Sign in</h1></main>',
        );

        const parseResult = await runCemDomCli(['parse', file]);
        const validateResult = await runCemDomCli(['validate', file]);

        assert.equal(parseResult.exitCode, 0);
        assert.match(parseResult.stdout, /"tagName": "main"/);
        assert.equal(validateResult.exitCode, 0);
        assert.match(validateResult.stdout, /No CEM DOM diagnostics/);
    });

    it('uses exit code 1 for validation errors', async () => {
        const dir = await mkdtemp(join(tmpdir(), 'cem-dom-'));
        const file = join(dir, 'fixture.html');
        await writeFile(file, '<main data-cem-screen="login" aria-labelledby="missing"></main>');

        const result = await runCemDomCli(['validate', file]);

        assert.equal(result.exitCode, 1);
        assert.match(result.stdout, /validate\.broken-reference/);
    });
});
