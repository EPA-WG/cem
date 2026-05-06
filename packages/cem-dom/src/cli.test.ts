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

    it('parses AST and event formats', async () => {
        const dir = await mkdtemp(join(tmpdir(), 'cem-dom-'));
        const file = join(dir, 'fixture.html');
        await writeFile(
            file,
            '<main data-cem-screen="login" aria-labelledby="title"><h1 id="title">Sign in</h1></main>',
        );

        const astResult = await runCemDomCli(['parse', file, '--format', 'ast']);
        const eventsResult = await runCemDomCli(['parse', file, '--format', 'events']);
        const ast = JSON.parse(astResult.stdout) as { type: string; children: Array<{ tagName: string }> };
        const events = JSON.parse(eventsResult.stdout) as Array<{ type: string; tagName?: string }>;

        assert.equal(astResult.exitCode, 0);
        assert.equal(ast.type, 'document');
        assert.equal(ast.children[0]?.tagName, 'main');
        assert.equal(eventsResult.exitCode, 0);
        assert.equal(events.some((event) => event.type === 'element-start' && event.tagName === 'main'), true);
    });

    it('converts parser output representations', async () => {
        const dir = await mkdtemp(join(tmpdir(), 'cem-dom-'));
        const file = join(dir, 'fixture.html');
        const out = join(dir, 'converted.events.json');
        await writeFile(
            file,
            '<main data-cem-screen="login" aria-labelledby="title"><h1 id="title">Sign in</h1></main>',
        );

        const astResult = await runCemDomCli(['convert', file, '--from-format', 'html', '--to-format', 'ast']);
        const aliasResult = await runCemDomCli(['convert', file, '--format', 'json']);
        const preservedOffsetsResult = await runCemDomCli([
            'convert',
            file,
            '--to-format',
            'ast',
            '--preserve-source-offsets',
        ]);
        const outResult = await runCemDomCli([
            'convert',
            file,
            '--from-format',
            'xml',
            '--to-format',
            'events',
            '--out',
            out,
        ]);

        const ast = JSON.parse(astResult.stdout) as {
            type: string;
            children: Array<{ tagName: string; location?: unknown }>;
        };
        const alias = JSON.parse(aliasResult.stdout) as { rootNodes: Array<{ type: string }> };
        const preservedOffsets = JSON.parse(preservedOffsetsResult.stdout) as {
            children: Array<{ location?: unknown }>;
        };
        const writtenEvents = JSON.parse(await readFile(out, 'utf8')) as Array<{ type: string; tagName?: string }>;

        assert.equal(astResult.exitCode, 0);
        assert.equal(ast.type, 'document');
        assert.equal(ast.children[0]?.tagName, 'main');
        assert.equal(ast.children[0]?.location, undefined);
        assert.equal(aliasResult.exitCode, 0);
        assert.equal(alias.rootNodes[0]?.type, 'element');
        assert.equal(preservedOffsetsResult.exitCode, 0);
        assert.equal(typeof preservedOffsets.children[0]?.location, 'object');
        assert.equal(outResult.exitCode, 0);
        assert.equal(outResult.stdout, '');
        assert.equal(writtenEvents.some((event) => event.type === 'element-start' && event.tagName === 'main'), true);
    });

    it('emits parser and validator trace output', async () => {
        const dir = await mkdtemp(join(tmpdir(), 'cem-dom-'));
        const file = join(dir, 'fixture.html');
        const warningFile = join(dir, 'warning.html');
        const out = join(dir, 'trace.txt');
        await writeFile(
            file,
            '<main data-cem-screen="login" aria-labelledby="title"><h1 id="title">Sign in</h1></main>',
        );
        await writeFile(warningFile, '<main data-cem-screen="login"></main>');

        const jsonResult = await runCemDomCli(['trace', file]);
        const textResult = await runCemDomCli(['trace', file, '--format', 'text', '--out', out]);
        const strictResult = await runCemDomCli(['trace', warningFile, '--fail-level', 'strict']);

        const trace = JSON.parse(jsonResult.stdout) as {
            uri: string;
            summary: { elementCount: number; validationDiagnosticCount: number };
            events: Array<{ stage: string; type: string; tagName?: string; diagnostic?: { code: string } }>;
        };
        const writtenTrace = await readFile(out, 'utf8');

        assert.equal(jsonResult.exitCode, 0);
        assert.equal(trace.summary.elementCount, 2);
        assert.equal(trace.summary.validationDiagnosticCount, 0);
        assert.equal(
            trace.events.some((event) => event.stage === 'parse' && event.type === 'element-start' && event.tagName === 'main'),
            true,
        );
        assert.equal(
            trace.events.some((event) => event.stage === 'validate' && event.type === 'validation-end'),
            true,
        );
        assert.equal(textResult.exitCode, 0);
        assert.equal(textResult.stdout, '');
        assert.match(writtenTrace, /Trace:/);
        assert.match(writtenTrace, /parse element-start/);
        assert.equal(strictResult.exitCode, 1);
        assert.match(strictResult.stdout, /validate\.missing-accessible-name/);
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

    it('inspects summary, tree, diagnostics, AST, and source offsets', async () => {
        const dir = await mkdtemp(join(tmpdir(), 'cem-dom-'));
        const file = join(dir, 'fixture.html');
        await writeFile(
            file,
            '<main data-cem-screen="login" aria-labelledby="title"><h1 id="title">Sign in</h1></main>',
        );

        const summaryResult = await runCemDomCli(['inspect', file]);
        const treeResult = await runCemDomCli(['inspect', file, '--show', 'tree']);
        const diagnosticsResult = await runCemDomCli(['inspect', file, '--show', 'diagnostics']);
        const astResult = await runCemDomCli(['inspect', file, '--show', 'ast']);
        const offsetsResult = await runCemDomCli(['inspect', file, '--show', 'source-offsets']);

        assert.equal(summaryResult.exitCode, 0);
        assert.match(summaryResult.stdout, /Elements: 2/);
        assert.match(summaryResult.stdout, /CEM attributes: data-cem-screen=1/);
        assert.equal(treeResult.exitCode, 0);
        assert.match(treeResult.stdout, /<main data-cem-screen="login">/);
        assert.equal(JSON.parse(diagnosticsResult.stdout).diagnostics.length, 0);
        assert.equal(JSON.parse(astResult.stdout).type, 'document');
        assert.equal(JSON.parse(offsetsResult.stdout).offsets[0].node, '<main>');
    });

    it('writes inspect output to --out', async () => {
        const dir = await mkdtemp(join(tmpdir(), 'cem-dom-'));
        const file = join(dir, 'fixture.html');
        const out = join(dir, 'inspect.json');
        await writeFile(file, '<main data-cem-screen="login" aria-label="Login"></main>');

        const result = await runCemDomCli(['inspect', file, '--format', 'json', '--out', out]);

        assert.equal(result.exitCode, 0);
        assert.equal(result.stdout, '');
        assert.equal(JSON.parse(await readFile(out, 'utf8')).elementCount, 1);
    });

    it('benchmarks parse and validate work', async () => {
        const dir = await mkdtemp(join(tmpdir(), 'cem-dom-'));
        const file = join(dir, 'fixture.html');
        const reportDir = join(dir, 'bench-report');
        await writeFile(file, '<main data-cem-screen="login" aria-label="Login"></main>');

        const textResult = await runCemDomCli(['bench', file, '--iterations', '2']);
        const jsonResult = await runCemDomCli(['bench', file, '--iterations', '2', '--format', 'json']);
        const reportResult = await runCemDomCli([
            'bench',
            file,
            '--iterations',
            '1',
            '--report-json',
            reportDir,
            '--profile',
            'cpu',
            '--cold-cache',
        ]);

        const jsonReport = JSON.parse(jsonResult.stdout) as {
            inputCount: number;
            iterations: number;
            averageInputMs: number;
        };
        const writtenReport = JSON.parse(await readFile(join(reportDir, 'cem-dom.bench.report.json'), 'utf8')) as {
            profile: string;
            coldCache: boolean;
        };

        assert.equal(textResult.exitCode, 0);
        assert.match(textResult.stdout, /Benchmarked 1 CEM DOM input/);
        assert.equal(jsonResult.exitCode, 0);
        assert.equal(jsonReport.inputCount, 1);
        assert.equal(jsonReport.iterations, 2);
        assert.equal(jsonReport.averageInputMs > 0, true);
        assert.equal(reportResult.exitCode, 0);
        assert.equal(writtenReport.profile, 'cpu');
        assert.equal(writtenReport.coldCache, true);
    });

    it('fails bench when the per-input budget is exceeded', async () => {
        const dir = await mkdtemp(join(tmpdir(), 'cem-dom-'));
        const file = join(dir, 'fixture.html');
        await writeFile(file, '<main data-cem-screen="login" aria-label="Login"></main>');

        const result = await runCemDomCli(['bench', file, '--iterations', '1', '--budget-ms', '0']);

        assert.equal(result.exitCode, 1);
        assert.match(result.stdout, /Budget: 0\.000ms per input \(exceeded\)/);
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

    it('roundtrips default and explicit fixtures as parser projections', async () => {
        const workspaceRoot = await mkdtemp(join(tmpdir(), 'cem-dom-workspace-'));
        const semanticDir = join(workspaceRoot, 'examples/semantic');
        const reports = join(workspaceRoot, 'reports');
        const out = join(workspaceRoot, 'roundtrip.md');
        await mkdir(semanticDir, { recursive: true });
        for (const fixture of ['assets-list', 'login', 'message-thread', 'profile', 'registration']) {
            await writeFile(
                join(semanticDir, `${fixture}.html`),
                `<main data-cem-screen="${fixture}" aria-label="${fixture}"></main>`,
            );
        }

        const defaultResult = await runCemDomCli(['fixture', 'roundtrip', '--to-format', 'events'], {
            workspaceRoot,
            cwd: workspaceRoot,
        });
        const defaultReport = JSON.parse(
            await readFile(join(workspaceRoot, 'packages/cem-dom/dist/cem-dom.roundtrip.report.json'), 'utf8'),
        ) as { targetFormat: string; inputCount: number; inputs: Array<{ outputSha256: string }> };
        const explicitResult = await runCemDomCli(
            [
                'fixture',
                'roundtrip',
                join(semanticDir, 'login.html'),
                '--to-format',
                'ast',
                '--format',
                'json',
                '--report-json',
                reports,
                '--report-md',
                reports,
            ],
            {
                workspaceRoot,
                cwd: workspaceRoot,
            },
        );
        const outResult = await runCemDomCli(
            [
                'fixture',
                'roundtrip',
                join(semanticDir, 'profile.html'),
                '--format',
                'markdown',
                '--out',
                out,
            ],
            {
                workspaceRoot,
                cwd: workspaceRoot,
            },
        );

        const explicitReport = JSON.parse(explicitResult.stdout) as {
            targetFormat: string;
            inputCount: number;
            summary: { passedCount: number };
            inputs: Array<{ elementCount: number; outputSha256: string }>;
        };

        assert.equal(defaultResult.exitCode, 0);
        assert.match(defaultResult.stdout, /Roundtripped 5 CEM DOM fixture/);
        assert.equal(defaultReport.targetFormat, 'events');
        assert.equal(defaultReport.inputCount, 5);
        assert.equal(defaultReport.inputs[0]?.outputSha256.length, 64);
        assert.equal(explicitResult.exitCode, 0);
        assert.equal(explicitReport.targetFormat, 'ast');
        assert.equal(explicitReport.inputCount, 1);
        assert.equal(explicitReport.summary.passedCount, 1);
        assert.equal(explicitReport.inputs[0]?.elementCount, 1);
        assert.equal(
            JSON.parse(await readFile(join(reports, 'cem-dom.roundtrip.report.json'), 'utf8')).targetFormat,
            'ast',
        );
        assert.match(await readFile(join(reports, 'cem-dom.roundtrip.report.md'), 'utf8'), /Fixture Roundtrip Report/);
        assert.equal(outResult.exitCode, 0);
        assert.equal(outResult.stdout, '');
        assert.match(await readFile(out, 'utf8'), /# CEM DOM Fixture Roundtrip Report/);
    });

    it('fails fixture roundtrip in strict mode when validation warnings are present', async () => {
        const dir = await mkdtemp(join(tmpdir(), 'cem-dom-'));
        const file = join(dir, 'warning.html');
        await writeFile(file, '<main data-cem-screen="login"></main>');

        const result = await runCemDomCli(['fixture', 'roundtrip', file, '--fail-level', 'strict']);

        assert.equal(result.exitCode, 1);
        assert.match(result.stdout, /validate\.missing-accessible-name/);
    });

    it('reports usage, I/O, and reserved command failures', async () => {
        const unknownOption = await runCemDomCli(['validate', 'fixture.html', '--unknown']);
        const invalidFailLevel = await runCemDomCli(['validate', 'fixture.html', '--fail-level', 'loud']);
        const invalidShow = await runCemDomCli(['inspect', 'fixture.html', '--show', 'scopes']);
        const invalidBenchIterations = await runCemDomCli(['bench', 'fixture.html', '--iterations', '0']);
        const invalidBenchProfile = await runCemDomCli(['bench', 'fixture.html', '--profile', 'gpu']);
        const invalidConvertFrom = await runCemDomCli(['convert', 'fixture.html', '--from-format', 'ast']);
        const invalidConvertTo = await runCemDomCli(['convert', 'fixture.html', '--to-format', 'html']);
        const invalidConvertAlias = await runCemDomCli(['convert', 'fixture.html', '--format', 'text']);
        const duplicateConvertFormat = await runCemDomCli([
            'convert',
            'fixture.html',
            '--format',
            'ast',
            '--to-format',
            'events',
        ]);
        const invalidTraceFormat = await runCemDomCli(['trace', 'fixture.html', '--format', 'markdown']);
        const invalidRoundtripFormat = await runCemDomCli([
            'fixture',
            'roundtrip',
            'fixture.html',
            '--format',
            'events',
        ]);
        const missingInput = await runCemDomCli(['parse']);
        const missingFile = await runCemDomCli(['validate', join(tmpdir(), 'does-not-exist.html')]);
        const reserved = await runCemDomCli(['transform', 'fixture.html']);

        assert.equal(unknownOption.exitCode, 2);
        assert.match(unknownOption.stderr, /Unknown option/);
        assert.equal(invalidFailLevel.exitCode, 2);
        assert.match(invalidFailLevel.stderr, /Invalid --fail-level/);
        assert.equal(invalidShow.exitCode, 2);
        assert.match(invalidShow.stderr, /Invalid --show/);
        assert.equal(invalidBenchIterations.exitCode, 2);
        assert.match(invalidBenchIterations.stderr, /--iterations must be an integer/);
        assert.equal(invalidBenchProfile.exitCode, 2);
        assert.match(invalidBenchProfile.stderr, /Invalid --profile/);
        assert.equal(invalidConvertFrom.exitCode, 2);
        assert.match(invalidConvertFrom.stderr, /Invalid --from-format/);
        assert.equal(invalidConvertTo.exitCode, 2);
        assert.match(invalidConvertTo.stderr, /Invalid --to-format/);
        assert.equal(invalidConvertAlias.exitCode, 2);
        assert.match(invalidConvertAlias.stderr, /convert supports --to-format/);
        assert.equal(duplicateConvertFormat.exitCode, 2);
        assert.match(duplicateConvertFormat.stderr, /Use either --to-format or --format/);
        assert.equal(invalidTraceFormat.exitCode, 2);
        assert.match(invalidTraceFormat.stderr, /trace supports --format json or text/);
        assert.equal(invalidRoundtripFormat.exitCode, 2);
        assert.match(invalidRoundtripFormat.stderr, /fixture roundtrip supports --format text, json, or markdown/);
        assert.equal(missingInput.exitCode, 2);
        assert.match(missingInput.stderr, /parse requires a file path/);
        assert.equal(missingFile.exitCode, 6);
        assert.match(missingFile.stderr, /ENOENT|no such file/i);
        assert.equal(reserved.exitCode, 2);
        assert.match(reserved.stderr, /reserved for a future Tier B\/C CLI release/);
    });
});
