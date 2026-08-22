import { generatedCommandSchema } from './generated/command-schema.js';
import type {
    ParsedCommandInvocationV1,
    ParsedCommandValueV1,
} from '@epa-wg/cem-ml/wasm';

export type CommandAvailability = 'available' | 'development-only' | 'unavailable';
export type CommandRuntime = 'native' | 'wasm-node' | 'wasm-browser-worker';
export type CommandArgumentAction =
    | 'set'
    | 'append'
    | 'set-true'
    | 'set-false'
    | 'count'
    | 'help'
    | 'help-short'
    | 'help-long'
    | 'version'
    | 'unknown';

export interface CommandRuntimeAvailability {
    readonly native: CommandAvailability;
    readonly wasmNode: CommandAvailability;
    readonly wasmBrowserWorker: CommandAvailability;
}

export interface CommandArgumentSchema {
    readonly id: string;
    readonly long: string | null;
    readonly short: string | null;
    readonly positionalIndex: number | null;
    readonly valueNames: readonly string[];
    readonly action: CommandArgumentAction;
    readonly minValues: number;
    readonly maxValues: number | null;
    readonly required: boolean;
    readonly global: boolean;
    readonly hidden: boolean;
    readonly allowHyphenValues: boolean;
    readonly valueDelimiter: string | null;
    readonly defaultValues: readonly string[];
    readonly possibleValues: readonly string[];
    readonly conflictsWith: readonly string[];
    readonly help: string | null;
    readonly longHelp: string | null;
}

export interface CommandArgumentGroupSchema {
    readonly id: string;
    readonly arguments: readonly string[];
    readonly required: boolean;
    readonly multiple: boolean;
}

export interface CommandDescriptorSchema {
    readonly name: string;
    readonly about: string | null;
    readonly longAbout: string | null;
    readonly capabilityOperation: string | null;
    readonly availability: CommandRuntimeAvailability;
    readonly arguments: readonly CommandArgumentSchema[];
    readonly groups: readonly CommandArgumentGroupSchema[];
    readonly subcommands: readonly CommandDescriptorSchema[];
}

export interface SharedCommandSchema {
    readonly schemaVersion: number;
    readonly commonVersion: string;
    readonly binaryName: string;
    readonly rootArguments: readonly CommandArgumentSchema[];
    readonly globalArguments: readonly CommandArgumentSchema[];
    readonly commands: readonly CommandDescriptorSchema[];
}

export type ParsedCommandValue = ParsedCommandValueV1;
export type ParsedCemMlCommand = ParsedCommandInvocationV1;

export interface ParseCemMlCommandOptions {
    readonly runtime?: CommandRuntime;
}

export const CEM_ML_CLI_COMMAND_CONTENT_TYPE = 'application/vnd.cem.cli-command+json' as const;
export const CEM_ML_CLI_COMMAND_SCHEMA = 'https://cem.dev/ns/cli/command/1' as const;

export interface CemMlCommandResourceV1 {
    readonly $schema: typeof CEM_ML_CLI_COMMAND_SCHEMA;
    readonly schemaVersion: 1;
    readonly commandSchemaVersion: number;
    readonly commonVersion: string;
    readonly binaryName: 'cem-ml';
    readonly argv: readonly string[];
}

export interface ParsedCemMlCommandResource {
    readonly resource: CemMlCommandResourceV1;
    readonly command: ParsedCemMlCommand;
}

export class CemMlCommandError extends Error {
    readonly code: string;
    readonly argumentId: string | undefined;

    constructor(code: string, message: string, argumentId?: string) {
        super(message);
        this.name = 'CemMlCommandError';
        this.code = code;
        this.argumentId = argumentId;
    }
}

export const commandSchema: SharedCommandSchema = generatedCommandSchema;

interface MutableParseState {
    readonly values: Map<string, ParsedCommandValue>;
    readonly provided: Set<string>;
    metaAction?: 'help' | 'version';
}

interface OptionToken {
    readonly argument: CommandArgumentSchema;
    readonly inlineValue?: string;
}

export function parseCemMlCommand(
    argv: readonly string[],
    parseOptions: ParseCemMlCommandOptions = {},
): ParsedCemMlCommand {
    const globalState = createState(commandSchema.globalArguments);
    const commandState: MutableParseState = { values: new Map(), provided: new Set() };
    const commandPath: string[] = [];
    let descriptor: CommandDescriptorSchema | undefined;
    let argumentsForCommand: readonly CommandArgumentSchema[] = commandSchema.rootArguments;
    let cursor = 0;
    let positionalOnly = false;

    while (cursor < argv.length) {
        const token = argv[cursor];
        if (token === undefined) break;
        if (token === '--' && descriptor !== undefined) {
            positionalOnly = true;
            cursor += 1;
            continue;
        }

        if (!positionalOnly && token.startsWith('-') && token !== '-') {
            const option = findOptionToken(token, commandSchema.globalArguments);
            if (option !== undefined) {
                cursor = consumeOption(argv, cursor, option, globalState);
                continue;
            }
            const localOption = findOptionToken(token, argumentsForCommand);
            if (localOption === undefined) {
                throw new CemMlCommandError(
                    'cem.command.unknown_option',
                    `unknown CEM-ML option \`${token}\``,
                );
            }
            cursor = consumeOption(argv, cursor, localOption, commandState);
            continue;
        }

        if (descriptor === undefined) {
            descriptor = commandSchema.commands.find((command) => command.name === token);
            if (descriptor === undefined) {
                throw new CemMlCommandError(
                    'cem.command.unknown_command',
                    `unknown CEM-ML command \`${token}\``,
                );
            }
            commandPath.push(descriptor.name);
            argumentsForCommand = descriptor.arguments;
            cursor += 1;
            continue;
        }

        const child = descriptor.subcommands.find((command) => command.name === token);
        if (child !== undefined && commandState.provided.size === 0) {
            descriptor = child;
            commandPath.push(child.name);
            argumentsForCommand = child.arguments;
            cursor += 1;
            continue;
        }

        consumePositional(token, argumentsForCommand, commandState);
        cursor += 1;
    }

    applyDefaults(commandSchema.globalArguments, globalState);
    applyDefaults(argumentsForCommand, commandState);
    const metaAction = commandState.metaAction ?? globalState.metaAction;
    if (descriptor === undefined && metaAction === undefined) {
        throw new CemMlCommandError('cem.command.required_command', 'a CEM-ML command is required');
    }
    if (metaAction === undefined) {
        validateArguments(commandSchema.globalArguments, [], globalState);
        if (descriptor !== undefined) {
            validateArguments(argumentsForCommand, descriptor.groups, commandState);
            validateRuntimeAvailability(descriptor, parseOptions.runtime);
        }
    }

    const { options, positionals } = partitionValues(argumentsForCommand, commandState.values);
    return Object.freeze({
        schemaVersion: commandSchema.schemaVersion,
        commonVersion: commandSchema.commonVersion,
        commandPath,
        globalOptions: freezeRecord(globalState.values),
        options,
        positionals,
        ...(metaAction === undefined ? {} : { metaAction }),
    });
}

export function serializeCemMlCommand(command: ParsedCemMlCommand): readonly string[] {
    if (command.schemaVersion !== commandSchema.schemaVersion) {
        throw new CemMlCommandError(
            'cem.command.schema_version',
            `command schema ${command.schemaVersion} is not supported; expected ${commandSchema.schemaVersion}`,
        );
    }
    const descriptor = resolveCommandPath(command.commandPath);
    const output: string[] = [];
    serializeOptions(output, commandSchema.globalArguments, command.globalOptions);
    output.push(...command.commandPath);
    if (descriptor === undefined) {
        serializeOptions(output, commandSchema.rootArguments, command.options);
        return Object.freeze(output);
    }
    const positionals = [...descriptor.arguments]
        .filter((argument) => argument.positionalIndex !== null)
        .sort((left, right) => (left.positionalIndex ?? 0) - (right.positionalIndex ?? 0));
    for (const argument of positionals) serializePositional(output, argument, command.positionals[argument.id]);
    serializeOptions(output, descriptor.arguments, command.options);
    return Object.freeze(output);
}

/** Parse one literal command line without shell expansion or evaluation. */
export function parseCemMlCommandText(
    source: string,
    parseOptions: ParseCemMlCommandOptions = {},
): ParsedCemMlCommand {
    const tokens = tokenizeCemMlCommandText(source);
    const binary = tokens[0];
    if (binary !== commandSchema.binaryName) {
        throw new CemMlCommandError(
            'cem.command.binary_name',
            binary === undefined
                ? `a ${commandSchema.binaryName} command is required`
                : `command must start with \`${commandSchema.binaryName}\`, received \`${binary}\``,
        );
    }
    return parseCemMlCommand(tokens.slice(1), parseOptions);
}

/** Serialize one normalized command with literal, POSIX-compatible quoting. */
export function serializeCemMlCommandText(command: ParsedCemMlCommand): string {
    return [commandSchema.binaryName, ...serializeCemMlCommand(command)]
        .map(quoteCemMlCommandTextToken)
        .join(' ');
}

/** Parse one portable authored command resource through the shared CLI grammar. */
export function parseCemMlCommandResource(
    source: string | Uint8Array,
    parseOptions: ParseCemMlCommandOptions = {},
): ParsedCemMlCommandResource {
    let value: unknown;
    try {
        const text = typeof source === 'string'
            ? source
            : new TextDecoder('utf-8', { fatal: true }).decode(source);
        value = JSON.parse(text);
    } catch (error) {
        throw new CemMlCommandError(
            'cem.cli_command.invalid_json',
            `CLI command JSON could not be parsed: ${error instanceof Error ? error.message : String(error)}`,
        );
    }
    const resource = validateCemMlCommandResource(value);
    const command = parseCemMlCommand(resource.argv, parseOptions);
    return Object.freeze({ resource, command });
}

/** Serialize authored configuration from a command without persisting its lowered run plan. */
export function serializeCemMlCommandResource(command: ParsedCemMlCommand): string {
    const resource: CemMlCommandResourceV1 = {
        $schema: CEM_ML_CLI_COMMAND_SCHEMA,
        schemaVersion: 1,
        commandSchemaVersion: commandSchema.schemaVersion,
        commonVersion: command.commonVersion,
        binaryName: 'cem-ml',
        argv: serializeCemMlCommand(command),
    };
    return `${JSON.stringify(resource, null, 2)}\n`;
}

function validateCemMlCommandResource(value: unknown): CemMlCommandResourceV1 {
    if (!value || typeof value !== 'object' || Array.isArray(value)) {
        throw new CemMlCommandError('cem.cli_command.invalid_json', 'CLI command JSON must be an object');
    }
    const candidate = value as Record<string, unknown>;
    const expected = new Set([
        '$schema',
        'schemaVersion',
        'commandSchemaVersion',
        'commonVersion',
        'binaryName',
        'argv',
    ]);
    const unknown = Object.keys(candidate).find((key) => !expected.has(key));
    if (unknown !== undefined) {
        throw new CemMlCommandError(
            'cem.cli_command.invalid_json',
            `CLI command JSON field \`${unknown}\` is unsupported`,
        );
    }
    if (candidate.$schema !== CEM_ML_CLI_COMMAND_SCHEMA) {
        throw new CemMlCommandError(
            'cem.cli_command.schema_identity_unsupported',
            `CLI command schema must be \`${CEM_ML_CLI_COMMAND_SCHEMA}\``,
        );
    }
    if (candidate.schemaVersion !== 1) {
        throw new CemMlCommandError(
            'cem.cli_command.schema_version_unsupported',
            'CLI command schemaVersion must be 1',
        );
    }
    if (candidate.commandSchemaVersion !== commandSchema.schemaVersion) {
        throw new CemMlCommandError(
            'cem.cli_command.command_schema_version_unsupported',
            `CLI command commandSchemaVersion must be ${commandSchema.schemaVersion}`,
        );
    }
    if (typeof candidate.commonVersion !== 'string' || !isCanonicalSemver(candidate.commonVersion)) {
        throw new CemMlCommandError(
            'cem.cli_command.common_version_invalid',
            'CLI command commonVersion must be canonical SemVer',
        );
    }
    if (candidate.binaryName !== commandSchema.binaryName) {
        throw new CemMlCommandError(
            'cem.cli_command.binary_name_invalid',
            `CLI command binaryName must be \`${commandSchema.binaryName}\``,
        );
    }
    if (!Array.isArray(candidate.argv) || candidate.argv.length === 0) {
        throw new CemMlCommandError('cem.cli_command.argv_empty', 'CLI command argv must contain an argument');
    }
    const argv = candidate.argv.map((argument, index) => {
        if (typeof argument !== 'string') {
            throw new CemMlCommandError(
                'cem.cli_command.invalid_json',
                `CLI command argv[${index}] must be text`,
            );
        }
        for (const character of argument) {
            const code = character.charCodeAt(0);
            if ((code < 0x20 && !['\t', '\n', '\r'].includes(character)) || code === 0x7f) {
                throw new CemMlCommandError(
                    'cem.cli_command.argument_control',
                    `CLI command argv[${index}] contains an unsupported control character`,
                );
            }
        }
        return argument;
    });
    return Object.freeze({
        $schema: CEM_ML_CLI_COMMAND_SCHEMA,
        schemaVersion: 1,
        commandSchemaVersion: candidate.commandSchemaVersion,
        commonVersion: candidate.commonVersion,
        binaryName: 'cem-ml',
        argv: Object.freeze(argv),
    });
}

function isCanonicalSemver(value: string): boolean {
    return /^(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)\.(0|[1-9][0-9]*)(?:-(?:0|[1-9][0-9]*|[0-9]*[A-Za-z-][0-9A-Za-z-]*)(?:\.(?:0|[1-9][0-9]*|[0-9]*[A-Za-z-][0-9A-Za-z-]*))*)?(?:\+[0-9A-Za-z-]+(?:\.[0-9A-Za-z-]+)*)?$/u.test(value);
}

function tokenizeCemMlCommandText(source: string): readonly string[] {
    if (typeof source !== 'string') {
        throw new TypeError('CEM-ML command text must be a string');
    }
    const tokens: string[] = [];
    let token = '';
    let tokenStarted = false;
    let quote: 'single' | 'double' | undefined;
    let escaping = false;
    for (let index = 0; index < source.length; index += 1) {
        const character = source[index] ?? '';
        const code = character.charCodeAt(0);
        if ((code < 0x20 && !['\t', '\n', '\r'].includes(character)) || code === 0x7f) {
            throw new CemMlCommandError(
                'cem.command.text_control',
                `command text contains an unsupported control character at offset ${index}`,
            );
        }
        if (escaping) {
            token += character;
            tokenStarted = true;
            escaping = false;
            continue;
        }
        if (quote === 'single') {
            if (character === "'") quote = undefined;
            else token += character;
            tokenStarted = true;
            continue;
        }
        if (quote === 'double') {
            if (character === '"') quote = undefined;
            else if (character === '\\') escaping = true;
            else token += character;
            tokenStarted = true;
            continue;
        }
        if (/\s/u.test(character)) {
            if (tokenStarted) {
                tokens.push(token);
                token = '';
                tokenStarted = false;
            }
            continue;
        }
        if (character === "'") {
            quote = 'single';
            tokenStarted = true;
        } else if (character === '"') {
            quote = 'double';
            tokenStarted = true;
        } else if (character === '\\') {
            escaping = true;
            tokenStarted = true;
        } else {
            token += character;
            tokenStarted = true;
        }
    }
    if (escaping) {
        throw new CemMlCommandError(
            'cem.command.text_escape',
            'command text ends with an incomplete escape',
        );
    }
    if (quote !== undefined) {
        throw new CemMlCommandError(
            'cem.command.text_quote',
            `command text contains an unterminated ${quote}-quoted argument`,
        );
    }
    if (tokenStarted) tokens.push(token);
    return Object.freeze(tokens);
}

function quoteCemMlCommandTextToken(token: string): string {
    if (/^[A-Za-z0-9_@%+=:,./~-]+$/u.test(token)) return token;
    return `'${token.replaceAll("'", "'\\''")}'`;
}

function createState(argumentsSchema: readonly CommandArgumentSchema[]): MutableParseState {
    const state: MutableParseState = { values: new Map(), provided: new Set() };
    applyDefaults(argumentsSchema, state);
    return state;
}

function findOptionToken(
    token: string,
    argumentsSchema: readonly CommandArgumentSchema[],
): OptionToken | undefined {
    if (token.startsWith('--')) {
        const equals = token.indexOf('=');
        const name = token.slice(2, equals < 0 ? undefined : equals);
        const argument = argumentsSchema.find((candidate) => candidate.long === name);
        if (argument === undefined) return undefined;
        return equals < 0 ? { argument } : { argument, inlineValue: token.slice(equals + 1) };
    }
    if (token.length === 2 && token.startsWith('-')) {
        const argument = argumentsSchema.find((candidate) => candidate.short === token[1]);
        return argument === undefined ? undefined : { argument };
    }
    return undefined;
}

function consumeOption(
    argv: readonly string[],
    cursor: number,
    token: OptionToken,
    state: MutableParseState,
): number {
    const { argument } = token;
    if (takesValues(argument)) {
        const values: string[] = [];
        if (token.inlineValue !== undefined) values.push(token.inlineValue);
        let next = cursor + 1;
        const maximum = argument.maxValues ?? Number.MAX_SAFE_INTEGER;
        while (values.length < maximum && next < argv.length) {
            const candidate = argv[next];
            if (candidate === undefined || (candidate.startsWith('-') && !argument.allowHyphenValues)) break;
            values.push(candidate);
            next += 1;
        }
        if (values.length < argument.minValues) {
            throw new CemMlCommandError(
                'cem.command.missing_value',
                `option \`${displayArgument(argument)}\` requires ${argument.minValues} value(s)`,
                argument.id,
            );
        }
        setValues(argument, splitValues(argument, values), state);
        return next;
    }
    if (token.inlineValue !== undefined) {
        throw new CemMlCommandError(
            'cem.command.unexpected_value',
            `flag \`${displayArgument(argument)}\` does not accept a value`,
            argument.id,
        );
    }
    setFlag(argument, state);
    return cursor + 1;
}

function consumePositional(
    token: string,
    argumentsSchema: readonly CommandArgumentSchema[],
    state: MutableParseState,
): void {
    const positionals = [...argumentsSchema]
        .filter((argument) => argument.positionalIndex !== null)
        .sort((left, right) => (left.positionalIndex ?? 0) - (right.positionalIndex ?? 0));
    const argument = positionals.find((candidate) => {
        const current = state.values.get(candidate.id);
        const count = Array.isArray(current) ? current.length : current === undefined ? 0 : 1;
        return candidate.maxValues === null || count < candidate.maxValues;
    });
    if (argument === undefined) {
        throw new CemMlCommandError(
            'cem.command.unexpected_positional',
            `unexpected positional argument \`${token}\``,
        );
    }
    setValues(argument, splitValues(argument, [token]), state);
}

function setValues(
    argument: CommandArgumentSchema,
    values: readonly string[],
    state: MutableParseState,
): void {
    validatePossibleValues(argument, values);
    if (argument.action === 'append' || argument.maxValues === null || (argument.maxValues ?? 0) > 1) {
        const current = state.provided.has(argument.id) ? state.values.get(argument.id) : undefined;
        const prior = Array.isArray(current) ? current : current === undefined ? [] : [String(current)];
        state.values.set(argument.id, [...prior, ...values]);
    } else {
        if (state.provided.has(argument.id)) argumentConflict(argument, argument.id);
        state.values.set(argument.id, values.length === 1 ? values[0] ?? '' : [...values]);
    }
    state.provided.add(argument.id);
}

function setFlag(argument: CommandArgumentSchema, state: MutableParseState): void {
    if (state.provided.has(argument.id) && argument.action !== 'count') {
        argumentConflict(argument, argument.id);
    }
    switch (argument.action) {
        case 'set-true':
            state.values.set(argument.id, true);
            break;
        case 'set-false':
            state.values.set(argument.id, false);
            break;
        case 'count':
            state.values.set(argument.id, Number(state.values.get(argument.id) ?? 0) + 1);
            break;
        case 'help':
        case 'help-short':
        case 'help-long':
            state.metaAction = 'help';
            state.values.set(argument.id, true);
            break;
        case 'version':
            state.metaAction = 'version';
            state.values.set(argument.id, true);
            break;
        default:
            throw new CemMlCommandError(
                'cem.command.schema_action',
                `unsupported action \`${argument.action}\` for \`${displayArgument(argument)}\``,
                argument.id,
            );
    }
    state.provided.add(argument.id);
}

function applyDefaults(
    argumentsSchema: readonly CommandArgumentSchema[],
    state: MutableParseState,
): void {
    for (const argument of argumentsSchema) {
        if (state.provided.has(argument.id)) continue;
        if (argument.defaultValues.length > 0) {
            state.values.set(
                argument.id,
                argument.action === 'append' || argument.defaultValues.length > 1
                    ? [...argument.defaultValues]
                    : argument.defaultValues[0] ?? '',
            );
        } else if (argument.action === 'set-true') {
            state.values.set(argument.id, false);
        } else if (argument.action === 'set-false') {
            state.values.set(argument.id, true);
        } else if (argument.action === 'count') {
            state.values.set(argument.id, 0);
        }
    }
}

function validateArguments(
    argumentsSchema: readonly CommandArgumentSchema[],
    groups: readonly CommandArgumentGroupSchema[],
    state: MutableParseState,
): void {
    for (const argument of argumentsSchema) {
        if (argument.required && !state.provided.has(argument.id) && argument.defaultValues.length === 0) {
            throw new CemMlCommandError(
                'cem.command.required_argument',
                `required argument \`${displayArgument(argument)}\` is missing`,
                argument.id,
            );
        }
        if (!state.provided.has(argument.id)) continue;
        for (const conflict of argument.conflictsWith) {
            if (state.provided.has(conflict)) argumentConflict(argument, conflict);
        }
    }
    for (const group of groups) {
        const present = group.arguments.filter((id) => state.provided.has(id));
        if (group.required && present.length === 0) {
            throw new CemMlCommandError(
                'cem.command.required_group',
                `one of ${group.arguments.map((id) => `\`${id}\``).join(', ')} is required`,
                group.id,
            );
        }
        if (!group.multiple && present.length > 1) {
            throw new CemMlCommandError(
                'cem.command.argument_conflict',
                `arguments ${present.map((id) => `\`${id}\``).join(', ')} conflict`,
                group.id,
            );
        }
    }
}

function validateRuntimeAvailability(
    descriptor: CommandDescriptorSchema,
    runtime: CommandRuntime | undefined,
): void {
    if (runtime === undefined) return;
    const availability =
        runtime === 'native'
            ? descriptor.availability.native
            : runtime === 'wasm-node'
              ? descriptor.availability.wasmNode
              : descriptor.availability.wasmBrowserWorker;
    if (availability === 'unavailable') {
        throw new CemMlCommandError(
            'cem.command.unavailable',
            `command \`${descriptor.name}\` is unavailable for ${runtime}`,
        );
    }
}

function validatePossibleValues(argument: CommandArgumentSchema, values: readonly string[]): void {
    if (argument.possibleValues.length === 0) return;
    for (const value of values) {
        if (!argument.possibleValues.includes(value)) {
            throw new CemMlCommandError(
                'cem.command.invalid_value',
                `\`${value}\` is not valid for \`${displayArgument(argument)}\`; expected ${argument.possibleValues.join(', ')}`,
                argument.id,
            );
        }
    }
}

function splitValues(argument: CommandArgumentSchema, values: readonly string[]): readonly string[] {
    return argument.valueDelimiter === null
        ? values
        : values.flatMap((value) => value.split(argument.valueDelimiter ?? ''));
}

function argumentConflict(argument: CommandArgumentSchema, conflict: string): never {
    throw new CemMlCommandError(
        'cem.command.argument_conflict',
        `argument \`${displayArgument(argument)}\` conflicts with \`${conflict}\``,
        argument.id,
    );
}

function takesValues(argument: CommandArgumentSchema): boolean {
    return argument.maxValues === null || argument.maxValues > 0;
}

function displayArgument(argument: CommandArgumentSchema): string {
    if (argument.long !== null) return `--${argument.long}`;
    if (argument.short !== null) return `-${argument.short}`;
    return argument.valueNames[0] ?? argument.id;
}

function partitionValues(
    argumentsSchema: readonly CommandArgumentSchema[],
    values: ReadonlyMap<string, ParsedCommandValue>,
): {
    readonly options: Readonly<Record<string, ParsedCommandValue>>;
    readonly positionals: Readonly<Record<string, ParsedCommandValue>>;
} {
    const options = new Map<string, ParsedCommandValue>();
    const positionals = new Map<string, ParsedCommandValue>();
    for (const argument of argumentsSchema) {
        const value = values.get(argument.id);
        if (value === undefined) continue;
        (argument.positionalIndex === null ? options : positionals).set(argument.id, value);
    }
    return { options: freezeRecord(options), positionals: freezeRecord(positionals) };
}

function freezeRecord(values: ReadonlyMap<string, ParsedCommandValue>): Readonly<Record<string, ParsedCommandValue>> {
    return Object.freeze(Object.fromEntries(values));
}

function resolveCommandPath(path: readonly string[]): CommandDescriptorSchema | undefined {
    let commands = commandSchema.commands;
    let descriptor: CommandDescriptorSchema | undefined;
    for (const name of path) {
        descriptor = commands.find((command) => command.name === name);
        if (descriptor === undefined) {
            throw new CemMlCommandError('cem.command.unknown_command', `unknown command path \`${path.join(' ')}\``);
        }
        commands = descriptor.subcommands;
    }
    return descriptor;
}

function serializeOptions(
    output: string[],
    argumentsSchema: readonly CommandArgumentSchema[],
    values: Readonly<Record<string, ParsedCommandValue>>,
): void {
    for (const argument of argumentsSchema) {
        if (argument.positionalIndex !== null) continue;
        const value = values[argument.id];
        if (value === undefined) continue;
        const flag = argument.long === null ? `-${argument.short ?? ''}` : `--${argument.long}`;
        if (argument.action === 'set-true' || argument.action.startsWith('help') || argument.action === 'version') {
            if (value === true) output.push(flag);
            continue;
        }
        if (argument.action === 'set-false') {
            if (value === false) output.push(flag);
            continue;
        }
        if (argument.action === 'count') {
            for (let index = 0; index < Number(value); index += 1) output.push(flag);
            continue;
        }
        const entries = Array.isArray(value) ? value : [String(value)];
        if (argument.action === 'append') {
            for (const entry of entries) output.push(flag, entry);
        } else if (entries.length > 0) {
            output.push(flag, ...entries);
        }
    }
}

function serializePositional(
    output: string[],
    argument: CommandArgumentSchema,
    value: ParsedCommandValue | undefined,
): void {
    if (value === undefined) return;
    if (typeof value !== 'string' && !Array.isArray(value)) {
        throw new CemMlCommandError(
            'cem.command.invalid_value',
            `positional \`${argument.id}\` must contain text`,
            argument.id,
        );
    }
    output.push(...(Array.isArray(value) ? value : [value]));
}
