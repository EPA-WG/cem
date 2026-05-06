import { mkdir, writeFile } from 'node:fs/promises';
import { dirname, extname, join } from 'node:path';
import { formatDiagnostics, type CemDiagnostic, type CemDomFailLevel } from './cem-dom.ts';

export interface CemDomReport {
    generatedAt: string;
    inputs: CemDomReportInput[];
    summary: CemDomReportSummary;
    options?: CemDomReportOptions;
}

export interface CemDomReportInput {
    uri: string;
    diagnostics: CemDiagnostic[];
}

export interface CemDomReportSummary {
    inputCount: number;
    infoCount: number;
    warningCount: number;
    errorCount: number;
    fatalCount: number;
    hardViolationCount: number;
}

export interface CemDomReportOptions {
    failLevel?: CemDomFailLevel;
    schema?: string;
    contentType?: string;
    baseUri?: string;
}

export interface CreateCemDomReportOptions extends CemDomReportOptions {
    generatedAt?: string;
}

export function normalizeDiagnostics(
    diagnostics: readonly CemDiagnostic[],
    sourceName: string,
): CemDiagnostic[] {
    return diagnostics.map((diagnostic) => ({
        ...diagnostic,
        uri: diagnostic.uri ?? sourceName,
        line: diagnostic.line ?? diagnostic.location?.line,
        column: diagnostic.column ?? diagnostic.location?.column,
        byteOffset: diagnostic.byteOffset ?? diagnostic.location?.offset,
    }));
}

export function createCemDomReport(
    inputs: readonly CemDomReportInput[],
    options: CreateCemDomReportOptions = {},
): CemDomReport {
    const normalizedInputs = inputs.map((input) => ({
        uri: input.uri,
        diagnostics: normalizeDiagnostics(input.diagnostics, input.uri),
    }));

    return {
        generatedAt: options.generatedAt ?? '1970-01-01T00:00:00.000Z',
        inputs: normalizedInputs,
        summary: summarizeDiagnostics(normalizedInputs),
        options: {
            failLevel: options.failLevel,
            schema: options.schema,
            contentType: options.contentType,
            baseUri: options.baseUri,
        },
    };
}

export function formatReportMarkdown(report: CemDomReport): string {
    const lines = [
        '# CEM DOM Report',
        '',
        `Generated: ${report.generatedAt}`,
        '',
        `Inputs: ${report.summary.inputCount}`,
        `Info: ${report.summary.infoCount}`,
        `Warnings: ${report.summary.warningCount}`,
        `Errors: ${report.summary.errorCount}`,
        `Fatal: ${report.summary.fatalCount}`,
        `Hard violations: ${report.summary.hardViolationCount}`,
        '',
    ];

    if (report.options) {
        lines.push('## Options', '');
        if (report.options.failLevel) {
            lines.push(`- Fail level: ${report.options.failLevel}`);
        }
        if (report.options.schema) {
            lines.push(`- Schema: ${report.options.schema}`);
        }
        if (report.options.contentType) {
            lines.push(`- Content type: ${report.options.contentType}`);
        }
        if (report.options.baseUri) {
            lines.push(`- Base URI: ${report.options.baseUri}`);
        }
        lines.push('');
    }

    for (const input of report.inputs) {
        lines.push(`## ${input.uri}`, '');
        lines.push('```txt');
        lines.push(formatDiagnostics(input.diagnostics));
        lines.push('```', '');
    }

    return `${lines.join('\n')}\n`;
}

export async function writeJsonReport(destination: string, report: CemDomReport): Promise<string> {
    const outputPath = resolveReportPath(destination, 'cem-dom.report.json', '.json');
    await mkdirForFile(outputPath);
    await writeFile(outputPath, `${JSON.stringify(report, null, 2)}\n`);
    return outputPath;
}

export async function writeMarkdownReport(destination: string, report: CemDomReport): Promise<string> {
    const outputPath = resolveReportPath(destination, 'cem-dom.report.md', '.md');
    await mkdirForFile(outputPath);
    await writeFile(outputPath, formatReportMarkdown(report));
    return outputPath;
}

function summarizeDiagnostics(inputs: readonly CemDomReportInput[]): CemDomReportSummary {
    const summary: CemDomReportSummary = {
        inputCount: inputs.length,
        infoCount: 0,
        warningCount: 0,
        errorCount: 0,
        fatalCount: 0,
        hardViolationCount: 0,
    };

    for (const input of inputs) {
        for (const diagnostic of input.diagnostics) {
            switch (diagnostic.severity) {
                case 'info':
                    summary.infoCount += 1;
                    break;
                case 'warning':
                    summary.warningCount += 1;
                    break;
                case 'error':
                    summary.errorCount += 1;
                    summary.hardViolationCount += 1;
                    break;
                case 'fatal':
                    summary.fatalCount += 1;
                    summary.hardViolationCount += 1;
                    break;
            }
        }
    }

    return summary;
}

function resolveReportPath(destination: string, defaultFileName: string, expectedExtension: string): string {
    return extname(destination) === expectedExtension ? destination : join(destination, defaultFileName);
}

async function mkdirForFile(filePath: string): Promise<void> {
    const directory = dirname(filePath);
    if (directory !== '.') {
        await mkdir(directory, { recursive: true });
    }
}
