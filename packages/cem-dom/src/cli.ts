#!/usr/bin/env node
import { readFile } from 'node:fs/promises';
import { basename } from 'node:path';
import { formatDiagnostics, parseCemDom, validateCemDom } from './index.ts';

export interface CemDomCliResult {
    exitCode: number;
    stdout: string;
    stderr: string;
}

const helpText = `Usage: cem-dom <command> [file]

Commands:
  parse <file>      Parse a CEM semantic document and print JSON.
  validate <file>   Validate a CEM semantic document.
  version           Print the package version.
  help              Print this help text.`;

export async function runCemDomCli(argv: readonly string[]): Promise<CemDomCliResult> {
    const [command, file] = argv;

    switch (command) {
        case 'parse': {
            if (!file) {
                return usageError('parse requires a file path.');
            }

            const source = await readFile(file, 'utf8');
            const document = parseCemDom(source, { sourceName: file });
            return {
                exitCode: document.diagnostics.some((diagnostic) => diagnostic.severity === 'error') ? 1 : 0,
                stdout: `${JSON.stringify(document, null, 2)}\n`,
                stderr: '',
            };
        }

        case 'validate': {
            if (!file) {
                return usageError('validate requires a file path.');
            }

            const source = await readFile(file, 'utf8');
            const diagnostics = validateCemDom(source, { sourceName: file });
            const hasErrors = diagnostics.some((diagnostic) => diagnostic.severity === 'error');
            return {
                exitCode: hasErrors ? 1 : 0,
                stdout: `${formatDiagnostics(diagnostics)}\n`,
                stderr: '',
            };
        }

        case 'version':
        case '--version':
        case '-v':
            return {
                exitCode: 0,
                stdout: `${await readPackageVersion()}\n`,
                stderr: '',
            };

        case 'help':
        case '--help':
        case '-h':
        case undefined:
            return {
                exitCode: 0,
                stdout: `${helpText}\n`,
                stderr: '',
            };

        default:
            return usageError(`Unknown command "${command}".`);
    }
}

async function readPackageVersion(): Promise<string> {
    const packageJson = JSON.parse(await readFile(new URL('../package.json', import.meta.url), 'utf8')) as {
        version?: string;
    };
    return packageJson.version ?? '0.0.0';
}

function usageError(message: string): CemDomCliResult {
    return {
        exitCode: 2,
        stdout: '',
        stderr: `${message}\n\n${helpText}\n`,
    };
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
