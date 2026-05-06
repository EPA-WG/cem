import { parseArgs } from 'node:util';
import type { CemDomFailLevel } from './cem-dom.ts';
import { isCemDomFailLevel } from './fail-level.ts';

export type CemDomCliFormat = 'text' | 'json' | 'markdown' | 'dom-json' | 'ast' | 'events' | 'tree';

export type CemDomInspectShow = 'summary' | 'ast' | 'diagnostics' | 'source-offsets' | 'tree';

export interface CemDomCliOptions {
    failLevel?: CemDomFailLevel;
    reportJson?: string;
    reportMd?: string;
    format?: CemDomCliFormat;
    out?: string;
    schema?: string;
    contentType?: string;
    baseUri?: string;
    show?: CemDomInspectShow;
    quiet: boolean;
    verbose: boolean;
    noColor: boolean;
    zeroHardViolations: boolean;
}

export type CemDomCliInvocation =
    | { kind: 'help'; options: CemDomCliOptions }
    | { kind: 'version'; options: CemDomCliOptions }
    | { kind: 'parse'; input: string; options: CemDomCliOptions }
    | { kind: 'validate'; inputs: string[]; options: CemDomCliOptions }
    | { kind: 'check'; inputs: string[]; options: CemDomCliOptions }
    | { kind: 'fixture-validate'; inputs: string[]; options: CemDomCliOptions }
    | { kind: 'inspect'; input: string; options: CemDomCliOptions }
    | { kind: 'reserved'; command: string; options: CemDomCliOptions }
    | { kind: 'usage-error'; message: string; options: CemDomCliOptions };

const validFormats = new Set<CemDomCliFormat>(['text', 'json', 'markdown', 'dom-json', 'ast', 'events', 'tree']);
const validInspectShows = new Set<CemDomInspectShow>(['summary', 'ast', 'diagnostics', 'source-offsets', 'tree']);
const reservedTopLevelCommands = new Set(['transform', 'convert', 'trace', 'bench']);
const reservedSchemaCommands = new Set(['emit', 'sample', 'replace']);
const reservedPluginCommands = new Set(['list', 'inspect', 'run']);

export function parseCemDomCliArgs(argv: readonly string[]): CemDomCliInvocation {
    const fallbackOptions = createCliOptions();

    try {
        const parsed = parseArgs({
            args: [...argv],
            allowPositionals: true,
            strict: true,
            options: {
                'fail-level': { type: 'string' },
                'report-json': { type: 'string' },
                'report-md': { type: 'string' },
                format: { type: 'string' },
                out: { type: 'string' },
                schema: { type: 'string' },
                'content-type': { type: 'string' },
                'base-uri': { type: 'string' },
                show: { type: 'string' },
                quiet: { type: 'boolean' },
                verbose: { type: 'boolean' },
                'no-color': { type: 'boolean' },
                'zero-hard-violations': { type: 'boolean' },
                help: { type: 'boolean', short: 'h' },
                version: { type: 'boolean', short: 'v' },
            },
        });

        const optionResult = readOptions(parsed.values);
        if ('message' in optionResult) {
            return {
                kind: 'usage-error',
                message: optionResult.message,
                options: fallbackOptions,
            };
        }

        const options = optionResult.options;
        const [command, subcommand, ...rest] = parsed.positionals;

        if (parsed.values.version === true) {
            return { kind: 'version', options };
        }

        if (parsed.values.help === true || command === undefined || command === 'help') {
            return { kind: 'help', options };
        }

        switch (command) {
            case 'version':
                return { kind: 'version', options };
            case 'parse':
                return parseSingleInputCommand('parse', subcommand, rest, options);
            case 'validate':
                return parseMultiInputCommand('validate', [subcommand, ...rest], options);
            case 'check':
                return parseMultiInputCommand('check', [subcommand, ...rest], options);
            case 'inspect':
                return parseSingleInputCommand('inspect', subcommand, rest, options);
            case 'fixture':
                return parseFixtureCommand(subcommand, rest, options);
            case 'schema':
                return parseSchemaCommand(subcommand, options);
            case 'plugin':
                return parsePluginCommand(subcommand, options);
            default:
                if (reservedTopLevelCommands.has(command)) {
                    return { kind: 'reserved', command, options };
                }
                return {
                    kind: 'usage-error',
                    message: `Unknown command "${command}".`,
                    options,
                };
        }
    } catch (error) {
        return {
            kind: 'usage-error',
            message: error instanceof Error ? error.message : 'Could not parse CLI arguments.',
            options: fallbackOptions,
        };
    }
}

function readOptions(values: ReturnType<typeof parseArgs>['values']):
    | { options: CemDomCliOptions }
    | { message: string } {
    const failLevel = stringValue(values['fail-level']);
    const format = stringValue(values.format);
    const show = stringValue(values.show);

    if (failLevel !== undefined && !isCemDomFailLevel(failLevel)) {
        return {
            message: `Invalid --fail-level "${failLevel}". Expected parse, validate, or strict.`,
        };
    }

    if (format !== undefined && !isCemDomCliFormat(format)) {
        return {
            message: `Invalid --format "${format}". Expected text, json, markdown, dom-json, ast, events, or tree.`,
        };
    }

    if (show !== undefined && !isCemDomInspectShow(show)) {
        return {
            message: `Invalid --show "${show}". Expected summary, ast, diagnostics, source-offsets, or tree.`,
        };
    }

    return {
        options: createCliOptions({
            failLevel,
            reportJson: stringValue(values['report-json']),
            reportMd: stringValue(values['report-md']),
            format,
            out: stringValue(values.out),
            schema: stringValue(values.schema),
            contentType: stringValue(values['content-type']),
            baseUri: stringValue(values['base-uri']),
            show,
            quiet: values.quiet === true,
            verbose: values.verbose === true,
            noColor: values['no-color'] === true,
            zeroHardViolations: values['zero-hard-violations'] === true,
        }),
    };
}

function createCliOptions(options: Partial<CemDomCliOptions> = {}): CemDomCliOptions {
    return {
        quiet: false,
        verbose: false,
        noColor: false,
        zeroHardViolations: false,
        ...options,
    };
}

function parseSingleInputCommand(
    kind: 'parse' | 'inspect',
    input: string | undefined,
    extraInputs: string[],
    options: CemDomCliOptions,
): CemDomCliInvocation {
    if (!input) {
        return {
            kind: 'usage-error',
            message: `${kind} requires a file path.`,
            options,
        };
    }

    if (extraInputs.length > 0) {
        return {
            kind: 'usage-error',
            message: `${kind} accepts exactly one file path.`,
            options,
        };
    }

    return { kind, input, options };
}

function parseMultiInputCommand(
    kind: 'validate' | 'check',
    inputs: Array<string | undefined>,
    options: CemDomCliOptions,
): CemDomCliInvocation {
    const filteredInputs = inputs.filter((input): input is string => input !== undefined);
    if (filteredInputs.length === 0) {
        return {
            kind: 'usage-error',
            message: `${kind} requires at least one file path.`,
            options,
        };
    }

    return { kind, inputs: filteredInputs, options };
}

function parseFixtureCommand(
    subcommand: string | undefined,
    inputs: string[],
    options: CemDomCliOptions,
): CemDomCliInvocation {
    if (subcommand === 'validate') {
        return {
            kind: 'fixture-validate',
            inputs,
            options,
        };
    }

    if (subcommand === 'roundtrip') {
        return { kind: 'reserved', command: 'fixture roundtrip', options };
    }

    return {
        kind: 'usage-error',
        message: 'fixture requires a subcommand: validate.',
        options,
    };
}

function parseSchemaCommand(
    subcommand: string | undefined,
    options: CemDomCliOptions,
): CemDomCliInvocation {
    if (subcommand && reservedSchemaCommands.has(subcommand)) {
        return { kind: 'reserved', command: `schema ${subcommand}`, options };
    }

    return {
        kind: 'usage-error',
        message: 'schema requires a reserved subcommand: emit, sample, or replace.',
        options,
    };
}

function parsePluginCommand(
    subcommand: string | undefined,
    options: CemDomCliOptions,
): CemDomCliInvocation {
    if (subcommand && reservedPluginCommands.has(subcommand)) {
        return { kind: 'reserved', command: `plugin ${subcommand}`, options };
    }

    return {
        kind: 'usage-error',
        message: 'plugin requires a reserved subcommand: list, inspect, or run.',
        options,
    };
}

function isCemDomCliFormat(value: string): value is CemDomCliFormat {
    return validFormats.has(value as CemDomCliFormat);
}

function isCemDomInspectShow(value: string): value is CemDomInspectShow {
    return validInspectShows.has(value as CemDomInspectShow);
}

function stringValue(value: unknown): string | undefined {
    return typeof value === 'string' ? value : undefined;
}
