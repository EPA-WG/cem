import { describe, it } from 'node:test';
import assert from 'node:assert/strict';
import { mkdir, mkdtemp, readFile, writeFile } from 'node:fs/promises';
import { join } from 'node:path';
import { tmpdir } from 'node:os';
import { runCemDomCli } from './cli.ts';

describe('runCemDomCli', () => {
    it('prints help and version', async () => {
        const dir = await mkdtemp(join(tmpdir(), 'cem-dom-'));
        await writeFile(join(dir, 'package.json'), '{"version":"9.8.7"}\n');

        const helpResult = await runCemDomCli(['--help']);
        const versionResult = await runCemDomCli(['--version'], { packageRoot: dir });

        assert.equal(helpResult.exitCode, 0);
        assert.match(helpResult.stdout, /fixture validate/);
        assert.equal(versionResult.exitCode, 0);
        assert.equal(versionResult.stdout, '9.8.7\n');
    });

    it('parses a file to stdout and to --out', async () => {
        const dir = await mkdtemp(join(tmpdir(), 'cem-dom-'));
        const file = join(dir, 'fixture.html');
        const out = join(dir, 'parsed.json');
        await writeFile(
            file,
            '<main data-cem-screen="login" aria-labelledby="title"><h1 id="title">Sign in</h1></main>',
        );

        const stdoutResult = await runCemDomCli(['parse', file]);
        const outResult = await runCemDomCli(['parse', file, '--out', out]);
        const written = await readFile(out, 'utf8');

        assert.equal(stdoutResult.exitCode, 0);
        assert.match(stdoutResult.stdout, /"tagName": "main"/);
        assert.equal(outResult.exitCode, 0);
        assert.equal(outResult.stdout, '');
        assert.match(written, /"tagName": "main"/);
    });

    it('validates a file with text, json, markdown, and report outputs', async () => {
        const dir = await mkdtemp(join(tmpdir(), 'cem-dom-'));
        const file = join(dir, 'fixture.html');
        const reports = join(dir, 'reports');
        await writeFile(
            file,
            '<main data-cem-screen="login" aria-labelledby="title"><h1 id="title">Sign in</h1></main>',
        );

        const textResult = await runCemDomCli(['validate', file]);
        const jsonResult = await runCemDomCli(['validate', file, '--format', 'json']);
        const markdownResult = await runCemDomCli(['validate', file, '--format', 'markdown']);
        const reportResult = await runCemDomCli([
            'validate',
            file,
            '--report-json',
            reports,
            '--report-md',
            reports,
        ]);

        assert.equal(textResult.exitCode, 0);
        assert.match(textResult.stdout, /No CEM DOM diagnostics/);
        assert.equal(jsonResult.exitCode, 0);
        assert.equal(JSON.parse(jsonResult.stdout).summary.inputCount, 1);
        assert.equal(markdownResult.exitCode, 0);
        assert.match(markdownResult.stdout, /# CEM DOM Report/);
        assert.equal(reportResult.exitCode, 0);
        assert.equal(JSON.parse(await readFile(join(reports, 'cem-dom.report.json'), 'utf8')).summary.inputCount, 1);
        assert.match(await readFile(join(reports, 'cem-dom.report.md'), 'utf8'), /Hard violations: 0/);
    });

    it('validates multiple files and aggregates report diagnostics', async () => {
        const dir = await mkdtemp(join(tmpdir(), 'cem-dom-'));
        const good = join(dir, 'good.html');
        const bad = join(dir, 'bad.html');
        await writeFile(good, '<main data-cem-screen="login" aria-label="Login"></main>');
        await writeFile(bad, '<main data-cem-screen="login" aria-labelledby="missing"></main>');

        const result = await runCemDomCli(['validate', good, bad, '--format', 'json']);
        const report = JSON.parse(result.stdout) as { summary: { inputCount: number; errorCount: number } };

        assert.equal(result.exitCode, 1);
        assert.equal(report.summary.inputCount, 2);
        assert.equal(report.summary.errorCount, 1);
    });

    it('fails strict validation on warnings', async () => {
        const dir = await mkdtemp(join(tmpdir(), 'cem-dom-'));
        const file = join(dir, 'warning.html');
        await writeFile(file, '<main data-cem-screen="login"></main>');

        const result = await runCemDomCli(['validate', file, '--fail-level', 'strict']);

        assert.equal(result.exitCode, 1);
        assert.match(result.stdout, /validate\.missing-accessible-name/);
    });

    it('runs check with zero hard violations policy', async () => {
        const dir = await mkdtemp(join(tmpdir(), 'cem-dom-'));
        const file = join(dir, 'broken.html');
        await writeFile(file, '<main data-cem-screen="login" aria-labelledby="missing"></main>');

        const result = await runCemDomCli(['check', file, '--zero-hard-violations']);

        assert.equal(result.exitCode, 1);
        assert.match(result.stdout, /validate\.broken-reference/);
    });

    it('validates default and explicit fixtures', async () => {
        const workspaceRoot = await mkdtemp(join(tmpdir(), 'cem-dom-workspace-'));
        const semanticDir = join(workspaceRoot, 'examples/semantic');
        await mkdir(semanticDir, { recursive: true });
        for (const fixture of ['assets-list', 'login', 'message-thread', 'profile', 'registration']) {
            await writeFile(
                join(semanticDir, `${fixture}.html`),
                `<main data-cem-screen="${fixture}" aria-label="${fixture}"></main>`,
            );
        }

        const defaultResult = await runCemDomCli(['fixture', 'validate'], {
            workspaceRoot,
            cwd: workspaceRoot,
        });
        const explicitResult = await runCemDomCli(['fixture', 'validate', join(semanticDir, 'login.html')], {
            workspaceRoot,
            cwd: workspaceRoot,
        });

        assert.equal(defaultResult.exitCode, 0);
        assert.match(defaultResult.stdout, /Validated 5 CEM DOM fixture/);
        assert.equal(explicitResult.exitCode, 0);
        assert.match(explicitResult.stdout, /Validated 1 CEM DOM fixture/);
        assert.equal(
            JSON.parse(await readFile(join(workspaceRoot, 'packages/cem-dom/dist/cem-dom.report.json'), 'utf8')).summary
                .inputCount,
            1,
        );
    });

    it('reports usage, I/O, and reserved command failures', async () => {
        const unknownOption = await runCemDomCli(['validate', 'fixture.html', '--unknown']);
        const invalidFailLevel = await runCemDomCli(['validate', 'fixture.html', '--fail-level', 'loud']);
        const missingInput = await runCemDomCli(['parse']);
        const missingFile = await runCemDomCli(['validate', join(tmpdir(), 'does-not-exist.html')]);
        const reserved = await runCemDomCli(['transform', 'fixture.html']);

        assert.equal(unknownOption.exitCode, 2);
        assert.match(unknownOption.stderr, /Unknown option/);
        assert.equal(invalidFailLevel.exitCode, 2);
        assert.match(invalidFailLevel.stderr, /Invalid --fail-level/);
        assert.equal(missingInput.exitCode, 2);
        assert.match(missingInput.stderr, /parse requires a file path/);
        assert.equal(missingFile.exitCode, 6);
        assert.match(missingFile.stderr, /ENOENT|no such file/i);
        assert.equal(reserved.exitCode, 2);
        assert.match(reserved.stderr, /reserved for a future Tier B\/C CLI release/);
    });
});
