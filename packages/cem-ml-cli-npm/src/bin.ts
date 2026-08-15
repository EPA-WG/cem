#!/usr/bin/env node

import { realpathSync } from 'node:fs';
import process from 'node:process';
import { pathToFileURL } from 'node:url';

import {
    CemMlCommandError,
    commandSchema,
    parseCemMlCommand,
    type CommandArgumentSchema,
    type CommandDescriptorSchema,
    type ParsedCemMlCommand,
} from './command.js';
import { NodeCommandServiceError, type NodeCommandServiceHandle } from './node-command.js';
import { createNodeCommandHost } from './node-host.js';
import { NodeCommandInvocationError } from './node-invocation.js';
import { createNodeCommandService } from './node-service.js';

const EXIT_USAGE = 2;
const EXIT_IO = 6;
const EXIT_INTERNAL = 7;

export async function runCemMlExecutable(argv: readonly string[]): Promise<number> {
    let parsed: ParsedCemMlCommand;
    try {
        parsed = parseCemMlCommand(argv, { runtime: 'wasm-node' });
    } catch (error) {
        await writeProcessError(error);
        return EXIT_USAGE;
    }

    if (parsed.metaAction === 'help') {
        try {
            await writeProcessStream(process.stdout, new TextEncoder().encode(renderHelp(parsed)));
            return 0;
        } catch (error) {
            await writeProcessError(error);
            return EXIT_IO;
        }
    }

    const abortController = new AbortController();
    const abort = (signal: NodeJS.Signals): void => {
        abortController.abort(new Error(`received ${signal}`));
    };
    const onInterrupt = (): void => abort('SIGINT');
    const onTerminate = (): void => abort('SIGTERM');
    process.once('SIGINT', onInterrupt);
    process.once('SIGTERM', onTerminate);

    const host = createNodeCommandHost({
        stdout: process.stdout,
        stderr: process.stderr,
        deferStreamCommits: true,
    });
    let service: Awaited<ReturnType<typeof createNodeCommandService>> | undefined;
    let handle: NodeCommandServiceHandle | undefined;
    try {
        service = await createNodeCommandService({ host });
        const command = parsed.metaAction === 'version' ? versionCommand(parsed) : parsed;
        const execution = await service.run(
            command,
            { stdoutIsTerminal: process.stdout.isTTY ?? false },
            { signal: abortController.signal },
        );
        handle = execution.handle;
        const result = await handle;
        await service.publish(execution.invocation, result);
        return result.exitCode ?? EXIT_INTERNAL;
    } catch (error) {
        await writeProcessError(error);
        return executableErrorCode(error);
    } finally {
        process.off('SIGINT', onInterrupt);
        process.off('SIGTERM', onTerminate);
        if (handle !== undefined) await handle.dispose().catch(() => undefined);
        if (service !== undefined) await service.close().catch(() => undefined);
    }
}

function versionCommand(parsed: ParsedCemMlCommand): ParsedCemMlCommand {
    return {
        schemaVersion: parsed.schemaVersion,
        commonVersion: parsed.commonVersion,
        commandPath: ['version'],
        globalOptions: parsed.globalOptions,
        options: {},
        positionals: {},
    };
}

function renderHelp(parsed: ParsedCemMlCommand): string {
    const descriptor = resolveDescriptor(parsed.commandPath);
    const name = ['cem-ml', ...parsed.commandPath].join(' ');
    const about = descriptor?.about ?? 'CEM-ML validation, query, and transformation tools';
    const argumentsSchema = descriptor?.arguments ?? commandSchema.rootArguments;
    const positionals = argumentsSchema
        .filter((argument) => argument.positionalIndex !== null)
        .sort((left, right) => (left.positionalIndex ?? 0) - (right.positionalIndex ?? 0));
    const options = [...commandSchema.globalArguments, ...argumentsSchema].filter(
        (argument) => argument.positionalIndex === null && !argument.hidden,
    );
    const usageArguments = positionals.map((argument) => {
        const value = argument.valueNames[0] ?? argument.id.toUpperCase();
        const repeated = argument.maxValues === null || argument.maxValues > 1 ? '...' : '';
        return argument.required ? `<${value}>${repeated}` : `[${value}]${repeated}`;
    });
    const lines = [`Usage: ${name} [OPTIONS]${usageArguments.length === 0 ? '' : ` ${usageArguments.join(' ')}`}`];
    if (descriptor === undefined) lines[0] += ' <COMMAND>';
    lines.push('', about);
    if (descriptor === undefined) {
        lines.push('', 'Commands:');
        for (const command of commandSchema.commands.filter((command) => !isHiddenCommand(command))) {
            lines.push(`  ${command.name.padEnd(12)}${command.about ?? ''}`);
        }
    }
    if (positionals.length > 0) {
        lines.push('', 'Arguments:');
        for (const argument of positionals) lines.push(formatHelpArgument(argument, true));
    }
    if (options.length > 0) {
        lines.push('', 'Options:');
        for (const option of options) lines.push(formatHelpArgument(option, false));
    }
    return `${lines.join('\n')}\n`;
}

function resolveDescriptor(path: readonly string[]): CommandDescriptorSchema | undefined {
    let commands = commandSchema.commands;
    let descriptor: CommandDescriptorSchema | undefined;
    for (const name of path) {
        descriptor = commands.find((command) => command.name === name);
        if (descriptor === undefined) return undefined;
        commands = descriptor.subcommands;
    }
    return descriptor;
}

function isHiddenCommand(command: CommandDescriptorSchema): boolean {
    return command.availability.wasmNode === 'unavailable';
}

function formatHelpArgument(argument: CommandArgumentSchema, positional: boolean): string {
    const valueName = argument.valueNames[0] ?? argument.id.toUpperCase();
    const label = positional
        ? `<${valueName}>`
        : [
              argument.short === null ? null : `-${argument.short}`,
              argument.long === null ? null : `--${argument.long}`,
          ]
              .filter((part): part is string => part !== null)
              .join(', ') + (argument.maxValues === 0 ? '' : ` <${valueName}>`);
    return `  ${label.padEnd(28)}${argument.help ?? ''}`;
}

function executableErrorCode(error: unknown): number {
    if (error instanceof CemMlCommandError) return EXIT_USAGE;
    if (error instanceof NodeCommandInvocationError) return error.exitCode;
    if (error instanceof NodeCommandServiceError) return EXIT_INTERNAL;
    if (isRecord(error) && typeof error.code === 'string') return EXIT_IO;
    return EXIT_INTERNAL;
}

async function writeProcessError(error: unknown): Promise<void> {
    const message = error instanceof Error ? error.message : String(error);
    await writeProcessStream(process.stderr, new TextEncoder().encode(`cem-ml: ${message}\n`)).catch(
        () => undefined,
    );
}

function writeProcessStream(stream: NodeJS.WriteStream, bytes: Uint8Array): Promise<void> {
    if (stream.write(bytes)) return Promise.resolve();
    return new Promise((resolve) => stream.once('drain', resolve));
}

function isRecord(value: unknown): value is Record<string, unknown> {
    return typeof value === 'object' && value !== null && !Array.isArray(value);
}

const entry = process.argv[1];
if (entry !== undefined && pathToFileURL(realpathSync(entry)).href === import.meta.url) {
    void runCemMlExecutable(process.argv.slice(2)).then(
        (exitCode) => {
            process.exitCode = exitCode;
        },
        async (error: unknown) => {
            await writeProcessError(error);
            process.exitCode = EXIT_INTERNAL;
        },
    );
}
