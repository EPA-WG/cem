#!/usr/bin/env node
import { mkdir, readFile, writeFile } from 'node:fs/promises';
import { basename, dirname, isAbsolute, relative, resolve } from 'node:path';
import { performance } from 'node:perf_hooks';
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
    type CemDomDocument,
    type CemDomElementNode,
    type CemDomFailLevel,
    type CemDomNode,
    type CemDomReport,
} from './index.ts';
import {
    parseCemDomCliArgs,
    type CemDomCliOptions,
    type CemDomConvertToFormat,
    type CemDomInspectShow,
} from './lib/cli-options.ts';

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

interface BenchInputResult {
    uri: string;
    bytes: number;
    parseMs: number;
    validateMs: number;
    diagnostics: CemDiagnostic[];
}

interface BenchReport {
    generatedAt: string;
    inputCount: number;
    iterations: number;
    profile?: string;
    coldCache: boolean;
    budgetMs?: number;
    budgetExceeded: boolean;
    totalMs: number;
    parseMs: number;
    validateMs: number;
    averageIterationMs: number;
    averageInputMs: number;
    inputs: BenchInputResult[];
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

Implemented commands:
  parse <input>              Parse one CEM semantic document and print DOM JSON.
  validate <input...>        Validate one or more CEM semantic documents.
  check <input...>           Parse + validate for CI-friendly checks.
  inspect <input>            Inspect parser-backed document structure.
  bench <input...>           Benchmark parser and validator performance.
  convert <input>            Convert parser output representation from HTML/XML input.
  fixture validate [input...] Validate semantic fixtures and write reports.
  version                    Print the package version.
  help                       Print this help text.

Reserved Tier B/C commands:
  transform, trace
  schema emit|sample|replace
  fixture roundtrip
  plugin list|inspect|run

Common options:
  --fail-level parse|validate|strict
  --format text|json|markdown|dom-json|ast|events|tree
  --from-format html|xml
  --to-format dom-json|ast|events
  --show summary|ast|diagnostics|source-offsets|tree
  --iterations <n>
  --budget-ms <n>
  --profile cpu|memory
  --cold-cache
  --preserve-source-offsets
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
            case 'inspect':
                return await runInspect(invocation.input, invocation.options, cwd);
            case 'bench':
                return await runBench(invocation.inputs, invocation.options, cwd);
            case 'convert':
                return await runConvert(invocation.input, invocation.options, cwd);
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
    if (format !== 'dom-json' && format !== 'json' && format !== 'ast' && format !== 'events') {
        return usageError('parse supports --format dom-json, json, ast, or events.');
    }

    const cliInput = await readCliInput(input, options, cwd);
    const document = parseCemDom(cliInput.source, {
        sourceName: cliInput.uri,
    });
    document.diagnostics = normalizeDiagnostics(document.diagnostics, cliInput.uri);
    const stdout = `${JSON.stringify(formatParsePayload(document, format), null, 2)}\n`;

    if (options.out) {
        await writeOutputFile(options.out, stdout);
    }

    return {
        exitCode: hasFailingDiagnostics(document.diagnostics, failLevel) ? 1 : 0,
        stdout: options.out || options.quiet ? '' : stdout,
        stderr: '',
    };
}

async function runConvert(input: string, options: CemDomCliOptions, cwd: string): Promise<CemDomCliResult> {
    const failLevel = options.failLevel ?? 'parse';
    const fromFormat = options.fromFormat ?? 'html';
    const toFormatResult = getConvertToFormat(options);

    if (typeof toFormatResult === 'string') {
        return usageError(toFormatResult);
    }

    const cliInput = await readCliInput(input, options, cwd);
    const document = parseConvertInput(cliInput.source, cliInput.uri, fromFormat);
    document.diagnostics = normalizeDiagnostics(document.diagnostics, cliInput.uri);

    const payload = formatConvertPayload(document, {
        toFormat: toFormatResult.toFormat,
        preserveSourceOffsets: options.preserveSourceOffsets,
    });
    const stdout = `${JSON.stringify(payload, null, 2)}\n`;

    if (options.out) {
        await writeOutputFile(options.out, stdout);
    }

    return {
        exitCode: hasFailingDiagnostics(document.diagnostics, failLevel) ? 1 : 0,
        stdout: options.out || options.quiet ? '' : stdout,
        stderr: '',
    };
}

async function runInspect(input: string, options: CemDomCliOptions, cwd: string): Promise<CemDomCliResult> {
    const cliInput = await readCliInput(input, options, cwd);
    const document = parseCemDom(cliInput.source, {
        sourceName: cliInput.uri,
    });
    document.diagnostics = normalizeDiagnostics(document.diagnostics, cliInput.uri);
    const diagnostics = normalizeDiagnostics(validateCemDom(cliInput.source, { sourceName: cliInput.uri }), cliInput.uri);
    const show = options.show ?? 'summary';
    const format = options.format ?? (show === 'tree' ? 'tree' : show === 'summary' ? 'text' : 'json');

    if (format !== 'text' && format !== 'json' && format !== 'tree') {
        return usageError('inspect supports --format text, json, or tree.');
    }

    const payload = createInspectPayload(show, document, diagnostics, cliInput.uri);
    const stdout = formatInspectPayload(show, format, payload);

    if (options.out) {
        await writeOutputFile(options.out, stdout);
    }

    return {
        exitCode: 0,
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

    if (format !== 'text' && format !== 'json' && format !== 'markdown') {
        return usageError(`${command} supports --format text, json, or markdown.`);
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

async function runBench(
    inputs: readonly string[],
    options: CemDomCliOptions,
    cwd: string,
): Promise<CemDomCliResult> {
    const format = options.format ?? 'text';
    if (format !== 'text' && format !== 'json') {
        return usageError('bench supports --format text or json.');
    }

    const iterations = options.iterations ?? 10;
    if (!Number.isInteger(iterations) || iterations < 1) {
        return usageError('--iterations must be an integer greater than or equal to 1.');
    }

    const report = await createBenchReport(inputs, options, cwd, iterations);
    const stdout = formatBenchOutput(report, format);

    if (options.reportJson) {
        await writeBenchJsonReport(options.reportJson, report);
    }

    if (options.out) {
        await writeOutputFile(options.out, stdout);
    }

    return {
        exitCode: report.budgetExceeded ? 1 : 0,
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

async function createBenchReport(
    inputs: readonly string[],
    options: CemDomCliOptions,
    cwd: string,
    iterations: number,
): Promise<BenchReport> {
    const warmedInputs = options.coldCache
        ? undefined
        : await Promise.all(inputs.map((input) => readCliInput(input, options, cwd)));
    const perInput = new Map<string, BenchInputResult>();
    let totalParseMs = 0;
    let totalValidateMs = 0;
    const totalStart = performance.now();

    for (let iteration = 0; iteration < iterations; iteration += 1) {
        const iterationInputs = options.coldCache
            ? await Promise.all(inputs.map((input) => readCliInput(input, options, cwd)))
            : (warmedInputs ?? []);

        for (const cliInput of iterationInputs) {
            const parseStart = performance.now();
            parseCemDom(cliInput.source, { sourceName: cliInput.uri });
            const parseMs = performance.now() - parseStart;

            const validateStart = performance.now();
            const diagnostics = normalizeDiagnostics(
                validateCemDom(cliInput.source, { sourceName: cliInput.uri }),
                cliInput.uri,
            );
            const validateMs = performance.now() - validateStart;

            totalParseMs += parseMs;
            totalValidateMs += validateMs;

            const previous = perInput.get(cliInput.uri);
            if (previous) {
                previous.parseMs += parseMs;
                previous.validateMs += validateMs;
                previous.diagnostics = diagnostics;
            } else {
                perInput.set(cliInput.uri, {
                    uri: cliInput.uri,
                    bytes: Buffer.byteLength(cliInput.source, 'utf8'),
                    parseMs,
                    validateMs,
                    diagnostics,
                });
            }
        }
    }

    const totalMs = performance.now() - totalStart;
    const inputCount = inputs.length;
    const averageIterationMs = totalMs / iterations;
    const averageInputMs = totalMs / (iterations * inputCount);
    const budgetExceeded = options.budgetMs !== undefined && averageInputMs > options.budgetMs;

    return {
        generatedAt: '1970-01-01T00:00:00.000Z',
        inputCount,
        iterations,
        profile: options.profile,
        coldCache: options.coldCache,
        budgetMs: options.budgetMs,
        budgetExceeded,
        totalMs,
        parseMs: totalParseMs,
        validateMs: totalValidateMs,
        averageIterationMs,
        averageInputMs,
        inputs: [...perInput.values()].map((input) => ({
            ...input,
            parseMs: input.parseMs / iterations,
            validateMs: input.validateMs / iterations,
        })),
    };
}

function formatBenchOutput(report: BenchReport, format: 'text' | 'json'): string {
    if (format === 'json') {
        return `${JSON.stringify(report, null, 2)}\n`;
    }

    return [
        `Benchmarked ${report.inputCount} CEM DOM input(s) for ${report.iterations} iteration(s).`,
        `Average iteration: ${formatMs(report.averageIterationMs)}`,
        `Average input: ${formatMs(report.averageInputMs)}`,
        `Parse total: ${formatMs(report.parseMs)}`,
        `Validate total: ${formatMs(report.validateMs)}`,
        report.budgetMs === undefined
            ? 'Budget: none'
            : `Budget: ${formatMs(report.budgetMs)} per input (${report.budgetExceeded ? 'exceeded' : 'passed'})`,
        '',
    ].join('\n');
}

async function writeBenchJsonReport(destination: string, report: BenchReport): Promise<void> {
    const outputPath = destination.endsWith('.json') ? destination : resolve(destination, 'cem-dom.bench.report.json');
    await writeOutputFile(outputPath, `${JSON.stringify(report, null, 2)}\n`);
}

function formatMs(value: number): string {
    return `${value.toFixed(3)}ms`;
}

function formatParsePayload(
    document: CemDomDocument,
    format: 'json' | 'dom-json' | 'ast' | 'events',
): unknown {
    switch (format) {
        case 'json':
        case 'dom-json':
            return document;
        case 'ast':
            return createAstPayload(document);
        case 'events':
            return createEventPayload(document);
    }
}

function parseConvertInput(source: string, uri: string, fromFormat: 'html' | 'xml'): CemDomDocument {
    switch (fromFormat) {
        case 'html':
        case 'xml':
            return parseCemDom(source, { sourceName: uri });
    }
}

function getConvertToFormat(options: CemDomCliOptions): { toFormat: CemDomConvertToFormat } | string {
    const formatAlias = options.format === undefined ? undefined : convertCliFormatToConvertToFormat(options.format);

    if (options.toFormat !== undefined && formatAlias !== undefined) {
        return 'Use either --to-format or --format for convert output, not both.';
    }

    if (options.format !== undefined && formatAlias === undefined) {
        return 'convert supports --to-format dom-json, ast, or events. As an alias, --format supports dom-json, json, ast, or events.';
    }

    return { toFormat: options.toFormat ?? formatAlias ?? 'dom-json' };
}

function convertCliFormatToConvertToFormat(format: CemDomCliOptions['format']): CemDomConvertToFormat | undefined {
    switch (format) {
        case 'json':
        case 'dom-json':
            return 'dom-json';
        case 'ast':
            return 'ast';
        case 'events':
            return 'events';
        default:
            return undefined;
    }
}

function formatConvertPayload(
    document: CemDomDocument,
    options: {
        toFormat: CemDomConvertToFormat;
        preserveSourceOffsets: boolean;
    },
): unknown {
    const payload = formatParsePayload(document, options.toFormat);
    return options.preserveSourceOffsets ? payload : omitSourceLocations(payload);
}

function omitSourceLocations(value: unknown): unknown {
    if (Array.isArray(value)) {
        return value.map((item) => omitSourceLocations(item));
    }

    if (value === null || typeof value !== 'object') {
        return value;
    }

    return Object.fromEntries(
        Object.entries(value as Record<string, unknown>)
            .filter(([key]) => key !== 'location')
            .map(([key, entryValue]) => [key, omitSourceLocations(entryValue)]),
    );
}

function createAstPayload(document: CemDomDocument): unknown {
    return {
        type: 'document',
        sourceName: document.sourceName,
        diagnostics: document.diagnostics,
        children: document.rootNodes.map((node) => createAstNode(node)),
    };
}

function createAstNode(node: CemDomNode): unknown {
    if (node.type === 'text') {
        return {
            type: 'text',
            value: node.value,
            location: node.location,
        };
    }

    return {
        type: 'element',
        tagName: node.tagName,
        attributes: Object.fromEntries(node.attributes.map((attribute) => [attribute.name, attribute.value])),
        location: node.location,
        children: node.children.map((child) => createAstNode(child)),
    };
}

function createEventPayload(document: CemDomDocument): unknown[] {
    const events: unknown[] = [
        {
            type: 'document-start',
            sourceName: document.sourceName,
        },
    ];

    for (const node of document.rootNodes) {
        appendNodeEvents(node, events);
    }

    for (const diagnostic of document.diagnostics) {
        events.push({
            type: 'diagnostic',
            diagnostic,
        });
    }

    events.push({ type: 'document-end' });
    return events;
}

function appendNodeEvents(node: CemDomNode, events: unknown[]): void {
    if (node.type === 'text') {
        events.push({
            type: 'text',
            value: node.value,
            location: node.location,
        });
        return;
    }

    events.push({
        type: 'element-start',
        tagName: node.tagName,
        attributes: node.attributes,
        location: node.location,
    });

    for (const child of node.children) {
        appendNodeEvents(child, events);
    }

    events.push({
        type: 'element-end',
        tagName: node.tagName,
        location: node.location,
    });
}

function createInspectPayload(
    show: CemDomInspectShow,
    document: CemDomDocument,
    diagnostics: readonly CemDiagnostic[],
    uri: string,
): unknown {
    switch (show) {
        case 'summary':
            return createInspectSummary(document, diagnostics, uri);
        case 'ast':
            return createAstPayload(document);
        case 'diagnostics':
            return {
                uri,
                diagnostics,
            };
        case 'source-offsets':
            return {
                uri,
                offsets: document.elements.map((element) => ({
                    node: describeElement(element),
                    line: element.location.line,
                    column: element.location.column,
                    byteOffset: element.location.offset,
                })),
            };
        case 'tree':
            return renderTree(document);
    }
}

function createInspectSummary(
    document: CemDomDocument,
    diagnostics: readonly CemDiagnostic[],
    uri: string,
): unknown {
    const tagCounts = new Map<string, number>();
    const semanticAttributeCounts = new Map<string, number>();

    for (const element of document.elements) {
        tagCounts.set(element.tagName, (tagCounts.get(element.tagName) ?? 0) + 1);
        for (const attribute of element.attributes) {
            if (attribute.name.startsWith('data-cem-')) {
                semanticAttributeCounts.set(attribute.name, (semanticAttributeCounts.get(attribute.name) ?? 0) + 1);
            }
        }
    }

    return {
        uri,
        rootNodeCount: document.rootNodes.length,
        elementCount: document.elements.length,
        textNodeCount: countTextNodes(document.rootNodes),
        diagnosticCount: diagnostics.length,
        tagCounts: Object.fromEntries([...tagCounts.entries()].sort(([left], [right]) => left.localeCompare(right))),
        semanticAttributeCounts: Object.fromEntries(
            [...semanticAttributeCounts.entries()].sort(([left], [right]) => left.localeCompare(right)),
        ),
    };
}

function formatInspectPayload(show: CemDomInspectShow, format: 'text' | 'json' | 'tree', payload: unknown): string {
    if (format === 'json') {
        return `${JSON.stringify(payload, null, 2)}\n`;
    }

    if (format === 'tree') {
        return typeof payload === 'string' ? `${payload}\n` : `${JSON.stringify(payload, null, 2)}\n`;
    }

    if (show === 'summary' && isInspectSummary(payload)) {
        return [
            `URI: ${payload.uri}`,
            `Root nodes: ${payload.rootNodeCount}`,
            `Elements: ${payload.elementCount}`,
            `Text nodes: ${payload.textNodeCount}`,
            `Diagnostics: ${payload.diagnosticCount}`,
            `Tags: ${formatCounts(payload.tagCounts)}`,
            `CEM attributes: ${formatCounts(payload.semanticAttributeCounts)}`,
            '',
        ].join('\n');
    }

    if (show === 'tree' && typeof payload === 'string') {
        return `${payload}\n`;
    }

    return `${JSON.stringify(payload, null, 2)}\n`;
}

function renderTree(document: CemDomDocument): string {
    const lines: string[] = [];
    for (const node of document.rootNodes) {
        appendTreeLines(node, lines, 0);
    }
    return lines.join('\n');
}

function appendTreeLines(node: CemDomNode, lines: string[], depth: number): void {
    const indent = '  '.repeat(depth);
    if (node.type === 'text') {
        lines.push(`${indent}#text "${node.value}"`);
        return;
    }

    lines.push(`${indent}<${node.tagName}${formatTreeAttributes(node)}>`);
    for (const child of node.children) {
        appendTreeLines(child, lines, depth + 1);
    }
}

function formatTreeAttributes(element: CemDomElementNode): string {
    const semanticAttributes = element.attributes
        .filter((attribute) => attribute.name === 'id' || attribute.name.startsWith('data-cem-'))
        .map((attribute) => `${attribute.name}="${attribute.value}"`);
    return semanticAttributes.length > 0 ? ` ${semanticAttributes.join(' ')}` : '';
}

function countTextNodes(nodes: readonly CemDomNode[]): number {
    return nodes.reduce((count, node) => {
        if (node.type === 'text') {
            return count + 1;
        }
        return count + countTextNodes(node.children);
    }, 0);
}

function describeElement(element: CemDomElementNode): string {
    const id = element.attributes.find((attribute) => attribute.name === 'id')?.value;
    return id ? `<${element.tagName}#${id}>` : `<${element.tagName}>`;
}

function formatCounts(counts: Record<string, number>): string {
    const entries = Object.entries(counts);
    return entries.length === 0 ? '-' : entries.map(([key, value]) => `${key}=${value}`).join(', ');
}

function isInspectSummary(payload: unknown): payload is {
    uri: string;
    rootNodeCount: number;
    elementCount: number;
    textNodeCount: number;
    diagnosticCount: number;
    tagCounts: Record<string, number>;
    semanticAttributeCounts: Record<string, number>;
} {
    return typeof payload === 'object' && payload !== null && 'elementCount' in payload && 'tagCounts' in payload;
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
