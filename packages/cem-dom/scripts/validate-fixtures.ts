import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { relative, resolve } from 'node:path';
import { formatDiagnostics, validateCemDom, type CemDiagnostic } from '../src/index.ts';

interface FixtureReport {
    generatedAt: string;
    fixtureCount: number;
    errorCount: number;
    warningCount: number;
    fixtures: Array<{
        file: string;
        diagnostics: CemDiagnostic[];
    }>;
}

const workspaceRoot = resolve(import.meta.dirname, '../../..');
const fixtureFiles = [
    'examples/semantic/assets-list.html',
    'examples/semantic/login.html',
    'examples/semantic/message-thread.html',
    'examples/semantic/profile.html',
    'examples/semantic/registration.html',
];

const fixtures = await Promise.all(
    fixtureFiles.map(async (file) => {
        const absolutePath = resolve(workspaceRoot, file);
        const source = await readFile(absolutePath, 'utf8');
        return {
            file,
            diagnostics: validateCemDom(source, {
                sourceName: file,
            }),
        };
    }),
);

const report: FixtureReport = {
    generatedAt: new Date().toISOString(),
    fixtureCount: fixtures.length,
    errorCount: fixtures.reduce(
        (count, fixture) => count + fixture.diagnostics.filter((diagnostic) => diagnostic.severity === 'error').length,
        0,
    ),
    warningCount: fixtures.reduce(
        (count, fixture) => count + fixture.diagnostics.filter((diagnostic) => diagnostic.severity === 'warning').length,
        0,
    ),
    fixtures,
};

const outputDir = resolve(workspaceRoot, 'packages/cem-dom/dist');
await mkdir(outputDir, { recursive: true });
await writeFile(resolve(outputDir, 'cem-dom.report.json'), `${JSON.stringify(report, null, 2)}\n`);
await writeFile(resolve(outputDir, 'cem-dom.report.md'), renderMarkdownReport(report));

console.log(
    `Validated ${report.fixtureCount} CEM DOM fixture(s): ${report.errorCount} error(s), ${report.warningCount} warning(s).`,
);
console.log(`Report: ${relative(workspaceRoot, resolve(outputDir, 'cem-dom.report.md'))}`);

if (report.errorCount > 0) {
    process.exitCode = 1;
}

function renderMarkdownReport(report: FixtureReport): string {
    const lines = [
        '# CEM DOM Fixture Validation Report',
        '',
        `Generated: ${report.generatedAt}`,
        '',
        `Fixtures: ${report.fixtureCount}`,
        `Errors: ${report.errorCount}`,
        `Warnings: ${report.warningCount}`,
        '',
    ];

    for (const fixture of report.fixtures) {
        lines.push(`## ${fixture.file}`, '');
        lines.push('```txt');
        lines.push(formatDiagnostics(fixture.diagnostics));
        lines.push('```', '');
    }

    return `${lines.join('\n')}\n`;
}
