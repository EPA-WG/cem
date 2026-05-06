#!/usr/bin/env node
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { basename, dirname, isAbsolute, relative, resolve } from 'node:path';
import {
    createCemDomReport,
    formatDiagnostics,
    formatReportMarkdown,
    hasFailingDiagnostics,
    hasHardViolations,
    normalizeDiagnostics,
    parseCemDom,
    validateCemDom,
    writeJsonReport,
    writeMarkdownReport,
    type CemDiagnostic,
    type CemDomFailLevel,
    type CemDomReport,
} from './index.ts';
import { parseCemDomCliArgs, type CemDomCliOptions } from './lib/cli-options.ts';

export interface CemDomCliResult {
    exitCode: number;
    stdout: string;
    stderr: string;
}

export interface RunCemDomCliOptions {
    cwd?: string;
    packageRoot?: string;
    workspaceRoot?: string;
}

interface CliInputResult {
    input: string;
    absolutePath: string;
    uri: string;
    source: string;
}

interface ValidationResult {
    input: CliInputResult;
    diagnostics: CemDiagnostic[];
}

const defaultPackageRoot = resolve(import.meta.dirname, '..');
const defaultWorkspaceRoot = resolve(defaultPackageRoot, '../..');

const defaultFixtureInputs = [
    'examples/semantic/assets-list.html',
    'examples/semantic/login.html',
    'examples/semantic/message-thread.html',
    'examples/semantic/profile.html',
    'examples/semantic/registration.html',
];

const helpText = `Usage: cem-dom <command> [input...] [options]

Tier A commands:
  parse <input>              Parse one CEM semantic document and print DOM JSON.
  validate <input...>        Validate one or more CEM semantic documents.
  check <input...>           Parse + validate for CI-friendly checks.
  fixture validate [input...] Validate semantic fixtures and write reports.
  version                    Print the package version.
  help                       Print this help text.

Reserved Tier B/C commands:
  transform, convert, inspect, trace, bench
  schema emit|sample|replace
  fixture roundtrip
  plugin list|inspect|run

Common options:
  --fail-level parse|validate|strict
  --format text|json|markdown|dom-json
  --out <file>
  --report-json <file-or-dir>
  --report-md <file-or-dir>
  --schema <uri-or-file>
  --content-type <type>
  --base-uri <uri>
  --zero-hard-violations
  --quiet
  --verbose
  --no-color`;

export async function runCemDomCli(
    argv: readonly string[],
    runOptions: RunCemDomCliOptions = {},
): Promise<CemDomCliResult> {
    const packageRoot = runOptions.packageRoot ?? defaultPackageRoot;
    const workspaceRoot = runOptions.workspaceRoot ?? defaultWorkspaceRoot;
    const cwd = runOptions.cwd ?? process.cwd();
    const invocation = parseCemDomCliArgs(argv);

    try {
        switch (invocation.kind) {
            case 'help':
                return success(`${helpText}\n`, invocation.options);
            case 'version':
                return success(`${await readPackageVersion(packageRoot)}\n`, invocation.options);
            case 'parse':
                return await runParse(invocation.input, invocation.options, cwd);
            case 'validate':
                return await runValidateOrCheck('validate', invocation.inputs, invocation.options, cwd);
            case 'check':
                return await runValidateOrCheck('check', invocation.inputs, invocation.options, cwd);
            case 'fixture-validate':
                return await runFixtureValidate(invocation.inputs, invocation.options, cwd, workspaceRoot);
            case 'reserved':
                return usageError(`Command "${invocation.command}" is reserved for a future Tier B/C CLI release.`);
            case 'usage-error':
                return usageError(invocation.message);
        }
    } catch (error) {
        return internalError(error);
    }
}

async function runParse(input: string, options: CemDomCliOptions, cwd: string): Promise<CemDomCliResult> {
    const failLevel = options.failLevel ?? 'parse';
    const format = options.format ?? 'dom-json';
    if (format !== 'dom-json' && format !== 'json') {
        return usageError('parse supports --format dom-json or --format json.');
    }

    const cliInput = await readCliInput(input, options, cwd);
    const document = parseCemDom(cliInput.source, {
        sourceName: cliInput.uri,
    });
    document.diagnostics = normalizeDiagnostics(document.diagnostics, cliInput.uri);
    const stdout = `${JSON.stringify(document, null, 2)}\n`;

    if (options.out) {
        await writeOutputFile(options.out, stdout);
    }

    return {
        exitCode: hasFailingDiagnostics(document.diagnostics, failLevel) ? 1 : 0,
        stdout: options.out || options.quiet ? '' : stdout,
        stderr: '',
    };
}

async function runValidateOrCheck(
    command: 'validate' | 'check',
    inputs: readonly string[],
    options: CemDomCliOptions,
    cwd: string,
): Promise<CemDomCliResult> {
    const failLevel = options.failLevel ?? 'validate';
    const format = options.format ?? 'text';

    if (format === 'dom-json') {
        return usageError(`${command} does not support --format dom-json.`);
    }

    if (options.out && inputs.length > 1) {
        return usageError(`--out cannot be used with multiple ${command} inputs. Use --report-json or --report-md.`);
    }

    const validationResults = await validateInputs(inputs, options, cwd);
    const report = createReport(validationResults, options, failLevel);
    const stdout = formatCommandOutput(format, report, collectDiagnostics(validationResults));
    await writeRequestedReports(options, report);

    if (options.out) {
        await writeOutputFile(options.out, stdout);
    }

    const failing =
        options.zeroHardViolations && command === 'check'
            ? validationResults.some((result) => hasHardViolations(result.diagnostics))
            : validationResults.some((result) => hasFailingDiagnostics(result.diagnostics, failLevel));

    return {
        exitCode: failing ? 1 : 0,
        stdout: options.out || options.quiet ? '' : stdout,
        stderr: '',
    };
}

async function runFixtureValidate(
    inputs: readonly string[],
    options: CemDomCliOptions,
    cwd: string,
    workspaceRoot: string,
): Promise<CemDomCliResult> {
    const effectiveInputs = inputs.length > 0 ? [...inputs] : defaultFixtureInputs;
    const failLevel = options.failLevel ?? 'validate';
    const fixtureCwd = inputs.length > 0 ? cwd : workspaceRoot;
    const validationResults = await validateInputs(effectiveInputs, options, fixtureCwd);
    const report = createReport(validationResults, options, failLevel);

    await writeJsonReport(options.reportJson ?? resolve(workspaceRoot, 'packages/cem-dom/dist/cem-dom.report.json'), report);
    await writeMarkdownReport(options.reportMd ?? resolve(workspaceRoot, 'packages/cem-dom/dist/cem-dom.report.md'), report);

    const output = `Validated ${report.summary.inputCount} CEM DOM fixture(s): ${report.summary.hardViolationCount} hard violation(s), ${report.summary.warningCount} warning(s).\n`;
    const failing = validationResults.some((result) => hasFailingDiagnostics(result.diagnostics, failLevel));

    return {
        exitCode: failing ? 1 : 0,
        stdout: options.quiet ? '' : output,
        stderr: '',
    };
}

async function validateInputs(
    inputs: readonly string[],
    options: CemDomCliOptions,
    cwd: string,
): Promise<ValidationResult[]> {
    return Promise.all(
        inputs.map(async (input) => {
            const cliInput = await readCliInput(input, options, cwd);
            const diagnostics = normalizeDiagnostics(
                validateCemDom(cliInput.source, { sourceName: cliInput.uri }),
                cliInput.uri,
            );
            return { input: cliInput, diagnostics };
        }),
    );
}

async function readCliInput(input: string, options: CemDomCliOptions, cwd: string): Promise<CliInputResult> {
    const absolutePath = isAbsolute(input) ? input : resolve(cwd, input);
    const source = await readFile(absolutePath, 'utf8');
    return {
        input,
        absolutePath,
        uri: createInputUri(input, absolutePath, options, cwd),
        source,
    };
}

function createInputUri(input: string, absolutePath: string, options: CemDomCliOptions, cwd: string): string {
    const normalizedInput = isAbsolute(input) ? relative(cwd, absolutePath) : input;
    const pathLikeUri = normalizedInput.split('\\').join('/');

    if (!options.baseUri) {
        return pathLikeUri;
    }

    const baseUri = options.baseUri.endsWith('/') ? options.baseUri : `${options.baseUri}/`;
    try {
        return new URL(pathLikeUri, baseUri).toString();
    } catch {
        return `${baseUri}${pathLikeUri}`;
    }
}

function createReport(
    validationResults: readonly ValidationResult[],
    options: CemDomCliOptions,
    failLevel: CemDomFailLevel,
): CemDomReport {
    return createCemDomReport(
        validationResults.map((result) => ({
            uri: result.input.uri,
            diagnostics: result.diagnostics,
        })),
        {
            failLevel,
            schema: options.schema,
            contentType: options.contentType,
            baseUri: options.baseUri,
        },
    );
}

function collectDiagnostics(validationResults: readonly ValidationResult[]): CemDiagnostic[] {
    return validationResults.flatMap((result) => result.diagnostics);
}

function formatCommandOutput(
    format: 'text' | 'json' | 'markdown',
    report: CemDomReport,
    diagnostics: readonly CemDiagnostic[],
): string {
    switch (format) {
        case 'text':
            return `${formatDiagnostics(diagnostics)}\n`;
        case 'json':
            return `${JSON.stringify(report, null, 2)}\n`;
        case 'markdown':
            return formatReportMarkdown(report);
    }
}

async function writeRequestedReports(options: CemDomCliOptions, report: CemDomReport): Promise<void> {
    if (options.reportJson) {
        await writeJsonReport(options.reportJson, report);
    }

    if (options.reportMd) {
        await writeMarkdownReport(options.reportMd, report);
    }
}

async function writeOutputFile(outputPath: string, content: string): Promise<void> {
    await mkdir(dirname(outputPath), { recursive: true });
    await writeFile(outputPath, content);
}

async function readPackageVersion(packageRoot: string): Promise<string> {
    const packageJson = JSON.parse(await readFile(resolve(packageRoot, 'package.json'), 'utf8')) as {
        version?: string;
    };
    return packageJson.version ?? '0.0.0';
}

function success(stdout: string, options: CemDomCliOptions): CemDomCliResult {
    return {
        exitCode: 0,
        stdout: options.quiet ? '' : stdout,
        stderr: '',
    };
}

function usageError(message: string): CemDomCliResult {
    return {
        exitCode: 2,
        stdout: '',
        stderr: `${message}\n\n${helpText}\n`,
    };
}

function internalError(error: unknown): CemDomCliResult {
    const message = error instanceof Error ? error.message : 'Unexpected CEM DOM CLI failure.';
    const exitCode = isIoError(error) ? 6 : 7;
    return {
        exitCode,
        stdout: '',
        stderr: `${message}\n`,
    };
}

function isIoError(error: unknown): boolean {
    if (!(error instanceof Error)) {
        return false;
    }

    const code = 'code' in error ? error.code : undefined;
    return (
        code === 'ENOENT' ||
        code === 'EACCES' ||
        code === 'EPERM' ||
        code === 'EISDIR' ||
        code === 'ENOTDIR' ||
        code === 'EROFS'
    );
}

if (isMain()) {
    const result = await runCemDomCli(process.argv.slice(2));
    if (result.stdout) {
        process.stdout.write(result.stdout);
    }
    if (result.stderr) {
        process.stderr.write(result.stderr);
    }
    process.exitCode = result.exitCode;
}

function isMain(): boolean {
    return process.argv[1] !== undefined && basename(process.argv[1]) === basename(import.meta.filename);
}
